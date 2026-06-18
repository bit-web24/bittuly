use crate::email::send_otp_email;
use crate::models::{AuthUserResponse, CreateUserPayload, UpdateUserPayload, User};
use crate::repo_trait::UserRepo;
use rand::Rng;
use shared::jwt::{
    create_access_token, create_pending_token, create_refresh_token, decode_pending_token,
};
use uuid::Uuid;

type ServiceResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[allow(dead_code)]
pub async fn create_user(
    repo: &dyn UserRepo,
    mut payload: CreateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    payload.password = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)?;
    let user = repo.create_user(payload).await?;
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
    repo: &dyn UserRepo,
    mut payload: CreateUserPayload,
) -> ServiceResult<String> {
    // Reject if email is already taken so we don't send an OTP pointlessly.
    if repo.get_user_by_email(&payload.email).await?.is_some() {
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
    repo: &dyn UserRepo,
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

    let user = repo.create_user(payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn login(
    repo: &dyn UserRepo,
    email: &str,
    password: &str,
) -> ServiceResult<AuthUserResponse> {
    let user: User = repo
        .get_user_by_email(email)
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

pub async fn get_user_by_id(
    repo: &dyn UserRepo,
    user_id: Uuid,
) -> Result<Option<User>, sqlx::Error> {
    repo.get_user_by_id(user_id).await
}

pub async fn update_user(
    repo: &dyn UserRepo,
    user_id: Uuid,
    mut payload: UpdateUserPayload,
) -> ServiceResult<AuthUserResponse> {
    if let Some(ref plain) = payload.password {
        payload.password = Some(bcrypt::hash(plain, bcrypt::DEFAULT_COST)?);
    }
    let user = repo.update_user(user_id, payload).await?;
    let token = create_access_token(user.id)?;
    let refresh_token = create_refresh_token(user.id)?;

    Ok(AuthUserResponse {
        user,
        token,
        refresh_token,
    })
}

pub async fn delete_user(repo: &dyn UserRepo, user_id: Uuid) -> Result<(), sqlx::Error> {
    repo.delete_user(user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockUserRepo;
    use crate::otp_store;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;

    // Helper to setup env vars
    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "super-secret-test-key-12345");
            std::env::set_var("MODE", "development"); // ensures emails go to otp_store
        }
    }

    #[tokio::test]
    async fn test_login_success() {
        setup_env();
        let repo = MockUserRepo::new();

        // Seed a user
        repo.create_user(CreateUserPayload {
            username: "testuser".into(),
            email: "test@example.com".into(),
            password: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
        })
        .await
        .unwrap();

        // Attempt login
        let response = login(&repo, "test@example.com", "password123")
            .await
            .unwrap();
        assert_eq!(response.user.email, "test@example.com");
        assert!(!response.token.is_empty());
        assert!(!response.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_login_invalid_password() {
        setup_env();
        let repo = MockUserRepo::new();
        repo.create_user(CreateUserPayload {
            username: "testuser".into(),
            email: "test@example.com".into(),
            password: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
        })
        .await
        .unwrap();

        let err = login(&repo, "test@example.com", "wrongpass")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "invalid credentials");
    }

    #[tokio::test]
    async fn test_request_signup_success() {
        setup_env();
        let repo = MockUserRepo::new();
        let payload = CreateUserPayload {
            username: "newuser".into(),
            email: "new@example.com".into(),
            password: "password123".into(),
        };

        let token = request_signup(&repo, payload).await.unwrap();
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_request_signup_duplicate_email() {
        setup_env();
        let repo = MockUserRepo::new();
        repo.create_user(CreateUserPayload {
            username: "existing".into(),
            email: "dup@example.com".into(),
            password: "hashed".into(),
        })
        .await
        .unwrap();

        let payload = CreateUserPayload {
            username: "newuser".into(),
            email: "dup@example.com".into(),
            password: "password123".into(),
        };

        let err = request_signup(&repo, payload).await.unwrap_err();
        assert_eq!(err.to_string(), "email already registered");
    }

    #[tokio::test]
    async fn test_verify_otp_success() {
        setup_env();
        let repo = MockUserRepo::new();
        let email = "otp@example.com";

        let pending_token = request_signup(
            &repo,
            CreateUserPayload {
                username: "otpuser".into(),
                email: email.into(),
                password: "password123".into(),
            },
        )
        .await
        .unwrap();

        // Fetch the generated OTP from the dev store
        let stored_otp = otp_store::get_all()
            .into_iter()
            .find(|o| o.email == email)
            .expect("OTP should be in store")
            .otp;

        let response = verify_otp(&repo, &pending_token, &stored_otp)
            .await
            .unwrap();
        assert_eq!(response.user.email, email);
        assert_eq!(response.user.username, "otpuser");

        // Ensure user was actually persisted to repo
        let db_user = repo.get_user_by_email(email).await.unwrap().unwrap();
        assert_eq!(db_user.email, email);
    }

    #[tokio::test]
    async fn test_verify_otp_invalid_code() {
        setup_env();
        let repo = MockUserRepo::new();
        let email = "otp2@example.com";

        let pending_token = request_signup(
            &repo,
            CreateUserPayload {
                username: "otpuser2".into(),
                email: email.into(),
                password: "password123".into(),
            },
        )
        .await
        .unwrap();

        let err = verify_otp(&repo, &pending_token, "000000")
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "invalid OTP");
    }
}
