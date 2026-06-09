use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

type EmailResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Send a 6-digit OTP to `to_email` via Gmail SMTP.
///
/// Reads `SMTP_USER` and `SMTP_PASS` from the environment at call time
/// so the transport is always fresh (no stale config across restarts).
pub async fn send_otp_email(to_email: &str, otp: &str) -> EmailResult {
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
