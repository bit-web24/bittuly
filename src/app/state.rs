use crate::db::redis::RedisConn;
use tokio::sync::mpsc::UnboundedSender;

pub type ClickSender = UnboundedSender<String>;

pub struct AppState {
    pub tx: ClickSender,
    pub redis: RedisConn,
}

impl AppState {
    pub fn new(tx: ClickSender, redis: RedisConn) -> Self {
        Self { tx, redis }
    }
}
