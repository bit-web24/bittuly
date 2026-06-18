use deadpool_lapin::{Config, Pool, Runtime};

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
