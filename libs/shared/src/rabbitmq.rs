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
