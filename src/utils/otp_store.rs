use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::{OnceLock, RwLock};

/// A single OTP entry stored during development.
#[derive(Debug, Clone, Serialize)]
pub struct OtpEntry {
    pub email: String,
    pub otp: String,
    pub created_at: DateTime<Utc>,
}

/// Global in-memory OTP store — only populated when MODE=development.
/// Capped at 50 entries (FIFO) to avoid unbounded growth in long dev sessions.
static STORE: OnceLock<RwLock<Vec<OtpEntry>>> = OnceLock::new();

fn store() -> &'static RwLock<Vec<OtpEntry>> {
    STORE.get_or_init(|| RwLock::new(Vec::new()))
}

/// Insert a new OTP entry. Called by send_otp_email() in development mode.
pub fn store_otp(email: &str, otp: &str) {
    let entry = OtpEntry {
        email: email.to_string(),
        otp: otp.to_string(),
        created_at: Utc::now(),
    };
    if let Ok(mut guard) = store().write() {
        if guard.len() >= 50 {
            guard.remove(0); // drop the oldest
        }
        guard.push(entry);
    }
    tracing::info!("[DEV OTP] email={} otp={}", email, otp);
}

/// Return a clone of all stored OTP entries (newest last).
pub fn get_all() -> Vec<OtpEntry> {
    store().read().map(|g| g.clone()).unwrap_or_default()
}
