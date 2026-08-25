use async_trait::async_trait;
use deadpool_lapin::{Config, Pool, Runtime};
use lapin::{
    BasicProperties,
    options::BasicPublishOptions,
    types::{AMQPValue, FieldTable},
};
use std::collections::HashMap;

/// Initialize a RabbitMQ connection pool and pre-declare all required queues.
///
/// Returns an error if the pool cannot be created (invalid URL / config).
/// Queue declaration failures are logged as warnings and are non-fatal —
/// queues must already exist on the broker, or will be declared lazily.
pub async fn init_rabbitmq_pool(amqp_url: &str) -> Result<Pool, deadpool_lapin::CreatePoolError> {
    let cfg = Config {
        url: Some(amqp_url.to_string()),
        ..Default::default()
    };

    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    if let Ok(conn) = pool.get().await
        && let Ok(channel) = conn.create_channel().await
    {
        let options = lapin::options::QueueDeclareOptions {
            durable: true,
            ..Default::default()
        };

        let queues = [
            ("user_deleted_dlq", lapin::types::FieldTable::default()),
            ("user_deleted_queue", lapin::types::FieldTable::default()),
            ("click_events_queue", lapin::types::FieldTable::default()),
        ];

        for (name, args) in queues {
            if let Err(e) = channel.queue_declare(name, options, args).await {
                tracing::warn!(
                    "Failed to pre-declare queue '{}': {}. It may already exist or be declared lazily.",
                    name,
                    e
                );
            }
        }

        for delay in [3, 9, 27] {
            let mut retry_args = lapin::types::FieldTable::default();
            retry_args.insert(
                "x-message-ttl".into(),
                lapin::types::AMQPValue::LongInt(delay * 1000),
            );
            retry_args.insert(
                "x-dead-letter-exchange".into(),
                lapin::types::AMQPValue::LongString("".into()),
            );
            retry_args.insert(
                "x-dead-letter-routing-key".into(),
                lapin::types::AMQPValue::LongString("user_deleted_queue".into()),
            );

            let queue_name = format!("user_deleted_retry_{}s", delay);
            if let Err(e) = channel
                .queue_declare(
                    &queue_name,
                    lapin::options::QueueDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    retry_args,
                )
                .await
            {
                tracing::warn!("Failed to pre-declare retry queue '{}': {}", queue_name, e);
            }
        }
    } else {
        tracing::warn!(
            "RabbitMQ pool created but could not pre-declare queues on startup. Will retry lazily."
        );
    }

    Ok(pool)
}

#[async_trait]
pub trait EventPublisher: Send + Sync {
    async fn publish(&self, routing_key: &str, payload: &[u8]) -> Result<(), String>;
}

// 1. Production implementation using RabbitMQ
pub struct RabbitMqPublisher {
    pool: Pool,
}

impl RabbitMqPublisher {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EventPublisher for RabbitMqPublisher {
    async fn publish(&self, routing_key: &str, payload: &[u8]) -> Result<(), String> {
        // Fetch connection & channel
        let conn = self.pool.get().await.map_err(|e| e.to_string())?;
        let channel = conn.create_channel().await.map_err(|e| e.to_string())?;

        // Extract telemetry trace context so spans carry over to RabbitMQ queues
        let mut carrier = HashMap::new();
        crate::telemetry::inject_context(&mut carrier);

        let mut amqp_headers = FieldTable::default();
        for (k, v) in carrier {
            amqp_headers.insert(k.into(), AMQPValue::LongString(v.into()));
        }
        let props = BasicProperties::default().with_headers(amqp_headers);

        // Publish
        channel
            .basic_publish(
                "", // default exchange
                routing_key,
                BasicPublishOptions::default(),
                payload,
                props,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

pub struct MockEventPublisher;
#[async_trait]
impl EventPublisher for MockEventPublisher {
    async fn publish(&self, _routing_key: &str, _payload: &[u8]) -> Result<(), String> {
        Ok(()) // Silently swallow events during unit tests
    }
}
