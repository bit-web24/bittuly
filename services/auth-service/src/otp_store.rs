use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// A single OTP entry stored during development.
#[derive(Debug, Clone, Serialize)]
pub struct OtpEntry {
    pub email: String,
    pub otp: String,
    pub created_at: DateTime<Utc>,
}

/// Global in-memory OTP store — only populated when MODE=development.
///
/// Keyed by email so only the most recent OTP per address is kept.
/// Re-requesting a signup overwrites the previous entry automatically.
static STORE: OnceLock<RwLock<HashMap<String, OtpEntry>>> = OnceLock::new();

fn store() -> &'static RwLock<HashMap<String, OtpEntry>> {
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Insert (or overwrite) the latest OTP for the given email.
/// Called by send_otp_email() in development mode.
pub fn store_otp(email: &str, otp: &str) {
    let entry = OtpEntry {
        email: email.to_string(),
        otp: otp.to_string(),
        created_at: Utc::now(),
    };
    if let Ok(mut guard) = store().write() {
        guard.insert(email.to_string(), entry);
    }
    tracing::info!("[DEV OTP] email={} otp={}", email, otp);
}

/// Return all stored OTP entries, sorted by most recent first.
pub fn get_all() -> Vec<OtpEntry> {
    let entries = store()
        .read()
        .map(|g| g.values().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut sorted = entries;
    sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    sorted
}
