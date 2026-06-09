use crate::db::postgres::DbPool;
use crate::middlewares::jwt::{
    create_access_token, create_pending_token, create_refresh_token, decode_pending_token,
};
use crate::models::user::{AuthUserResponse, CreateUserPayload, UpdateUserPayload, User};
use crate::repository::user_repository;
use crate::utils::email::send_otp_email;
use rand::Rng;
use uuid::Uuid;

type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn create_user(
    db: &DbPool,
    mut payload: CreateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    payload.password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)?;
    let user = user_repository::create_user(db, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

// ---------------------------------------------------------------------------
// OTP signup flow
// ---------------------------------------------------------------------------

/// Step 1 — validate + hash credentials, generate OTP, send email, return pending JWT.
///
/// The pending JWT embeds { email, username, password_hash, otp_hash } and
/// expires in 10 minutes. No user row is created yet.
pub async fn request_signup(
    db: &DbPool,
    mut payload: CreateUserPayload,
) -> ServiceResult<String> {
    // Reject if email is already taken so we don't send an OTP pointlessly.
    if user_repository::get_user_by_email(db, &payload.email).await?.is_some() {
        return Err("email already registered".into());
    }

    // Hash the password before it goes anywhere near a JWT.
    payload.password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)?;

    // Generate a 6-digit OTP and bcrypt-hash it for the pending token.
    let otp: String = rand::rng()
        .random_range(100_000u32..=999_999u32)
        .to_string();
    let otp_hash = bcrypt::hash(&otp, bcrypt::DEFAULT_COST)?;

    // Fire the email first — if it fails we return early without issuing a token.
    send_otp_email(&payload.email, &otp).await?;

    // Embed all pending data into a short-lived signed JWT.
    let pending_token = create_pending_token(
        &payload.email,
        &payload.username,
        &payload.password,
        &otp_hash,
    )?;

    Ok(pending_token)
}

/// Step 2 — verify the OTP, create the real user row, issue auth JWTs.
///
/// `pending_token` is the JWT returned by `request_signup`.
/// `otp` is the 6-digit code the user received by email.
pub async fn verify_otp(
    db: &DbPool,
    pending_token: &str,
    otp: &str,
) -> ServiceResult<AuthUserResponse> {
    // Decode and verify the pending JWT (signature + expiry checked by the library).
    let claims = decode_pending_token(pending_token)?;

    // Verify the submitted OTP against the hashed one inside the token.
    if !bcrypt::verify(otp, &claims.otp_hash)? {
        return Err("invalid OTP".into());
    }

    // OTP is valid — build the CreateUserPayload with the already-hashed password.
    let payload = CreateUserPayload {
        username: claims.username,
        email: claims.email,
        password: claims.password_hash, // already bcrypt-hashed in request_signup
    };

    let user = user_repository::create_user(db, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn login(db: &DbPool, email: &str, password: &str) -> ServiceResult<AuthUserResponse> {
    let user: User = user_repository::get_user_by_email(db, email)
        .await?
        .ok_or("invalid credentials")?;

    if !bcrypt::verify(password, &user.password)? {
        return Err("invalid credentials".into());
    }

    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn get_user_by_id(db: &DbPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    user_repository::get_user_by_id(db, user_id).await
}

pub async fn update_user(
    db: &DbPool,
    user_id: Uuid,
    mut payload: UpdateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    if let Some(ref plain) = payload.password {
        payload.password = Some(bcrypt::hash(plain, bcrypt::DEFAULT_COST)?);
    }
    let user = user_repository::update_user(db, user_id, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn delete_user(db: &DbPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    user_repository::delete_user(db, user_id).await
}
