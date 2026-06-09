use crate::utils::otp_store;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

type EmailResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Send a 6-digit OTP to `to_email`.
///
/// Behaviour is controlled by the `MODE` environment variable:
/// - `development` → stores OTP in the in-memory DebugOtpStore (no real email sent)
/// - anything else  → sends a real email via Gmail SMTP
pub async fn send_otp_email(to_email: &str, otp: &str) -> EmailResult {
    let mode = std::env::var("MODE").unwrap_or_default();

    if mode == "development" {
        otp_store::store_otp(to_email, otp);
        return Ok(());
    }

    // --- Production: real Gmail SMTP ---
    let smtp_user = std::env::var("SMTP_USER")?;
    let smtp_pass = std::env::var("SMTP_PASS")?;

    let email = Message::builder()
        .from(format!("Bittuly <{smtp_user}>").parse()?)
        .to(to_email.parse()?)
        .subject("Your Bittuly verification code")
        .header(ContentType::TEXT_PLAIN)
        .body(format!(
            "Your one-time verification code is: {otp}\n\nThis code expires in 10 minutes.\nIf you did not request this, please ignore this email."
        ))?;

    let creds = Credentials::new(smtp_user, smtp_pass);

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")?
        .credentials(creds)
        .build();

    mailer.send(email).await?;

    Ok(())
}

