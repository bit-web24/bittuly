use deadpool_lapin::{Config, Pool, Runtime};

pub async fn init_rabbitmq_pool(amqp_url: &str) -> Pool {
    let cfg = Config {
        url: Some(amqp_url.to_string()),
        ..Default::default()
    };

    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create RabbitMQ pool");

    if let Ok(conn) = pool.get().await
        && let Ok(channel) = conn.create_channel().await
    {
        let options = lapin::options::QueueDeclareOptions {
            durable: true,
            ..Default::default()
        };

        let _ = channel
            .queue_declare(
                "user_deleted_dlq",
                options,
                lapin::types::FieldTable::default(),
            )
            .await;

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

            let _ = channel
                .queue_declare(
                    &format!("user_deleted_retry_{}s", delay),
                    lapin::options::QueueDeclareOptions {
                        durable: true,
                        ..Default::default()
                    },
                    retry_args,
                )
                .await;
        }

        let _ = channel
            .queue_declare(
                "user_deleted_queue",
                options,
                lapin::types::FieldTable::default(),
            )
            .await;
        let _ = channel
            .queue_declare(
                "click_events_queue",
                options,
                lapin::types::FieldTable::default(),
            )
            .await;
    }

    pool
}
