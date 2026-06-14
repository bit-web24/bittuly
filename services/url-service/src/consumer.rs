use crate::repository as url_repository;
use shared::postgres::DbPool;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

pub fn spawn_consumer(mut rx: UnboundedReceiver<String>, consumer_db: DbPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut batch: HashMap<String, u64> = HashMap::new();
        let mut total_clicks: u64 = 0;

        let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        // Consume the immediate first tick so the timer starts 30 s from now,
        // not from the moment the task spawns.
        flush_interval.tick().await;

        loop {
            tokio::select! {
                // ── Arm 1: new click event from the channel ──────────────────
                maybe_code = rx.recv() => {
                    match maybe_code {
                        Some(short_code) => {
                            *batch.entry(short_code).or_insert(0) += 1;
                            total_clicks += 1;

                            if total_clicks >= 17 {
                                match url_repository::increment_click_counts(&consumer_db, &batch).await {
                                    Ok(()) => tracing::info!(total_clicks, "click batch flushed (size trigger)"),
                                    Err(e) => tracing::error!("click batch flush failed: {e}"),
                                }
                                batch.clear();
                                total_clicks = 0;
                            }
                        }
                        None => {
                            // Channel closed (server shutting down) — drain remainder
                            if !batch.is_empty() {
                                match url_repository::increment_click_counts(&consumer_db, &batch).await {
                                    Ok(()) => tracing::info!(total_clicks, "click batch flushed (shutdown drain)"),
                                    Err(e) => tracing::error!("click batch final flush failed: {e}"),
                                }
                            }
                            break;
                        }
                    }
                }

                // ── Arm 2: periodic 30-second flush ──────────────────────────
                _ = flush_interval.tick() => {
                    if !batch.is_empty() {
                        match url_repository::increment_click_counts(&consumer_db, &batch).await {
                            Ok(()) => tracing::info!(total_clicks, "click batch flushed (interval trigger)"),
                            Err(e) => tracing::error!("click batch interval flush failed: {e}"),
                        }
                        batch.clear();
                        total_clicks = 0;
                    }
                }
            }
        }
    })
}
