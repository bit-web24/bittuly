use crate::redis::RedisConn;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

pub type ClickSender = UnboundedSender<String>;

pub struct AppState {
    pub tx: ClickSender,
    pub redis: RedisConn,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(tx: ClickSender, redis: RedisConn) -> Self {
        Self {
            tx,
            redis,
            started_at: Instant::now(),
        }
    }
}
