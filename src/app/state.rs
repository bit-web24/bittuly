use tokio::sync::mpsc::UnboundedSender;

pub type ClickSender = UnboundedSender<String>;

pub struct AppState {
    pub tx: ClickSender,
}

impl AppState {
    pub fn from(tx: ClickSender) -> Self {
        Self { tx }
    }
}
