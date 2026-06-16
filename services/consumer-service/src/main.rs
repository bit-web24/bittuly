use ::redis::AsyncCommands;
use futures_lite::stream::StreamExt;
use lapin::{
    Connection, ConnectionProperties,
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
};
use serde::Deserialize;
use shared::postgres;
use shared::redis as shared_redis;
use std::collections::HashMap;
use std::env;
use tokio::time::{Duration, interval};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
struct UserDeletedEvent {
    user_id: Uuid,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "consumer_service=info,shared=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("URL_DATABASE_URL").expect("URL_DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    let amqp_url = env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set");

    let db = postgres::init_pg_pool(&db_url)
        .await
        .expect("Failed to connect to Database");
    let mut redis_conn = shared_redis::init_redis(&redis_url)
        .await
        .expect("Failed to connect to Redis");

    tracing::info!("Connecting to RabbitMQ...");
    let rabbit_conn = Connection::connect(&amqp_url, ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");

    let channel = rabbit_conn.create_channel().await?;

    // Declare queues as durable (required by RabbitMQ 4.0+)
    let options = QueueDeclareOptions {
        durable: true,
        ..Default::default()
    };

    let _ = channel
        .queue_declare("user_deleted_dlq", options, FieldTable::default())
        .await?;

    for delay in [3, 9, 27] {
        let mut retry_args = FieldTable::default();
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
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                retry_args,
            )
            .await?;
    }

    channel
        .queue_declare("click_events_queue", options, FieldTable::default())
        .await?;

    channel
        .queue_declare("user_deleted_queue", options, FieldTable::default())
        .await?;

    let addr_port = amqp_url
        .split('@')
        .next_back()
        .unwrap_or("127.0.0.1:5672")
        .split('/')
        .next()
        .unwrap_or("127.0.0.1:5672");

    println!("Consumer service listening on {}", addr_port);
    tracing::info!("Consumer service started and connected to RabbitMQ.");

    // Click Events Consumer
    let click_db = db.clone();
    let click_channel = rabbit_conn.create_channel().await?;
    let mut click_consumer = click_channel
        .basic_consume(
            "click_events_queue",
            "consumer_service_clicks",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tokio::spawn(async move {
        let mut batch: HashMap<String, u64> = HashMap::new();
        let mut total_clicks: u64 = 0;
        let mut last_delivery_tag: Option<u64> = None;
        let mut consecutive_failures = 0;
        let mut flush_interval = interval(Duration::from_secs(30));
        flush_interval.tick().await;

        loop {
            tokio::select! {
                maybe_delivery = click_consumer.next() => {
                    if let Some(Ok(delivery)) = maybe_delivery
                        && let Ok(short_code) = String::from_utf8(delivery.data.clone())
                    {
                            *batch.entry(short_code).or_insert(0) += 1;
                            total_clicks += 1;

                            last_delivery_tag = Some(delivery.delivery_tag);

                            if total_clicks >= 17 {
                                match increment_click_counts(&click_db, &batch).await {
                                    Ok(_) => {
                                        let unique_urls: Vec<_> = batch.keys().cloned().collect();
                                        consecutive_failures = 0;
                                        tracing::info!(
                                            "🚀 [CLICK BATCH] Flushed {} clicks across {} unique URLs (Trigger: Size). Codes: {:?}",
                                            total_clicks,
                                            unique_urls.len(),
                                            unique_urls
                                        );
                                        if let Some(tag) = last_delivery_tag.take() {
                                            let _ = click_channel.basic_ack(tag, lapin::options::BasicAckOptions { multiple: true }).await;
                                        }
                                    }
                                    Err(e) => {
                                        let delay = 3_u64.pow(consecutive_failures.min(2) + 1);
                                        consecutive_failures += 1;
                                        tracing::error!("Click batch flush failed (DB down?). Backing off for {}s. Error: {}", delay, e);
                                        tokio::time::sleep(Duration::from_secs(delay)).await;
                                        if let Some(tag) = last_delivery_tag.take() {
                                            let _ = click_channel.basic_nack(tag, lapin::options::BasicNackOptions { multiple: true, requeue: true }).await;
                                        }
                                    }
                                }
                                batch.clear();
                                total_clicks = 0;
                        }
                    }
                }
                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        match increment_click_counts(&click_db, &batch).await {
                            Ok(_) => {
                                let unique_urls: Vec<_> = batch.keys().cloned().collect();
                                consecutive_failures = 0;
                                tracing::info!(
                                    "⏱️  [CLICK BATCH] Flushed {} clicks across {} unique URLs (Trigger: Timer). Codes: {:?}",
                                    total_clicks,
                                    unique_urls.len(),
                                    unique_urls
                                );
                                if let Some(tag) = last_delivery_tag.take() {
                                    let _ = click_channel.basic_ack(tag, lapin::options::BasicAckOptions { multiple: true }).await;
                                }
                            }
                            Err(e) => {
                                let delay = 3_u64.pow(consecutive_failures.min(2) + 1);
                                consecutive_failures += 1;
                                tracing::error!("Click batch flush failed (DB down?). Backing off for {}s. Error: {}", delay, e);
                                tokio::time::sleep(Duration::from_secs(delay)).await;
                                if let Some(tag) = last_delivery_tag.take() {
                                    let _ = click_channel.basic_nack(tag, lapin::options::BasicNackOptions { multiple: true, requeue: true }).await;
                                }
                            }
                        }
                        batch.clear();
                        total_clicks = 0;
                    }
                }
            }
        }
    });

    // User Deleted Events Consumer
    let user_db = db.clone();
    let user_channel = rabbit_conn.create_channel().await?;
    let mut user_consumer = user_channel
        .basic_consume(
            "user_deleted_queue",
            "consumer_service_users",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tokio::spawn(async move {
        while let Some(Ok(delivery)) = user_consumer.next().await {
            if let Ok(event) = serde_json::from_slice::<UserDeletedEvent>(&delivery.data) {
                tracing::info!(
                    "🗑️  [USER DELETED] Received cleanup event for user_id: {}",
                    event.user_id
                );

                // Delete from Postgres and get deleted short codes atomically
                match sqlx::query_scalar::<_, String>(
                    "DELETE FROM urls WHERE user_id = $1 RETURNING short_code",
                )
                .bind(event.user_id)
                .fetch_all(&user_db)
                .await
                {
                    Ok(short_codes) => {
                        tracing::info!(
                            "✅ [CLEANUP COMPLETE] Wiped {} URLs from DB and Redis for user_id: {}. Affected codes: {:?}",
                            short_codes.len(),
                            event.user_id,
                            short_codes
                        );

                        // 3. Evict from Redis
                        for code in short_codes {
                            let cache_key = format!("url:{}", code);
                            let _: () = redis_conn.del(&cache_key).await.unwrap_or_default();
                        }

                        // 4. Manual ACK
                        let _ = delivery.ack(BasicAckOptions::default()).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to delete URLs for user_id {}: {}",
                            event.user_id,
                            e
                        );

                        let mut retry_count = 0;
                        if let Some(headers) = delivery.properties.headers()
                            && let Some(lapin::types::AMQPValue::LongInt(count)) =
                                headers.inner().get("x-retry-count")
                        {
                            retry_count = *count;
                        }

                        if retry_count >= 3 {
                            tracing::error!(
                                "Message exceeded 3 retries. Moving to user_deleted_dlq."
                            );
                            let _ = user_channel
                                .basic_publish(
                                    "",
                                    "user_deleted_dlq",
                                    lapin::options::BasicPublishOptions::default(),
                                    &delivery.data,
                                    delivery.properties.clone(),
                                )
                                .await;
                        } else {
                            let delay_secs = 3_i32.pow(retry_count as u32 + 1);
                            let target_queue = format!("user_deleted_retry_{}s", delay_secs);

                            tracing::warn!(
                                "Retrying message in {}s. Attempt: {}/3. Moving to {}.",
                                delay_secs,
                                retry_count + 1,
                                target_queue
                            );
                            let mut props = delivery.properties.clone();
                            let mut headers = props.headers().clone().unwrap_or_default();
                            headers.insert(
                                "x-retry-count".into(),
                                lapin::types::AMQPValue::LongInt(retry_count + 1),
                            );
                            props = props.with_headers(headers);

                            let _ = user_channel
                                .basic_publish(
                                    "",
                                    &target_queue,
                                    lapin::options::BasicPublishOptions::default(),
                                    &delivery.data,
                                    props,
                                )
                                .await;
                        }

                        let _ = delivery.ack(BasicAckOptions::default()).await;
                    }
                }
            } else {
                // Invalid format, just ack to discard
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        }
    });

    // Block forever so that the services (click consumer and user deletion consumer) keeps running
    std::future::pending::<()>().await;
    Ok(())
}

async fn increment_click_counts(
    db: &sqlx::PgPool,
    batch: &HashMap<String, u64>,
) -> Result<(), sqlx::Error> {
    let (codes, deltas): (Vec<String>, Vec<i64>) = batch
        .iter()
        .map(|(code, &count)| (code.clone(), count as i64))
        .unzip();

    sqlx::query(
        "UPDATE urls \
         SET click_count = click_count + d.delta \
         FROM (SELECT unnest($1::text[]) AS code, \
                      unnest($2::bigint[]) AS delta) AS d \
         WHERE urls.short_code = d.code",
    )
    .bind(&codes)
    .bind(&deltas)
    .execute(db)
    .await?;

    Ok(())
}
