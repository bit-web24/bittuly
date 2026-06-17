use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::ErrorKind,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const ACCESS_TOKEN_TYPE: &str = "access";
const REFRESH_TOKEN_TYPE: &str = "refresh";
const PENDING_TOKEN_TYPE: &str = "pending";
const ACCESS_TOKEN_TTL_SECONDS: u64 = 60 * 15; // 15 minutes
const REFRESH_TOKEN_TTL_SECONDS: u64 = 60 * 60 * 24 * 30; // 30 days
const PENDING_TOKEN_TTL_SECONDS: u64 = 60 * 10; // 10 minutes
const COOKIE_ACCESS: &str = "access_token";
const COOKIE_REFRESH: &str = "refresh_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub exp: usize,
    pub token_type: String,
}

fn create_token(
    user_id: Uuid,
    token_type: &str,
    ttl_seconds: u64,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let secret = std::env::var("JWT_SECRET")?;
    let exp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + ttl_seconds;
    let claims = Claims {
        sub: user_id,
        exp: exp as usize,
        token_type: token_type.to_string(),
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

pub fn create_access_token(
    user_id: Uuid,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    create_token(user_id, ACCESS_TOKEN_TYPE, ACCESS_TOKEN_TTL_SECONDS)
}

pub fn create_refresh_token(
    user_id: Uuid,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    create_token(user_id, REFRESH_TOKEN_TYPE, REFRESH_TOKEN_TTL_SECONDS)
}

// ---------------------------------------------------------------------------
// Pending signup JWT — carries encrypted signup data until OTP is verified
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpClaims {
    /// Identifies this as a pending-signup token, not an auth token.
    pub token_type: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub otp_hash: String,
    pub exp: usize,
}

/// Issue a short-lived JWT that holds the pending user's data + OTP hash.
/// The client stores this and sends it back with the OTP code.
pub fn create_pending_token(
    email: &str,
    username: &str,
    password_hash: &str,
    otp_hash: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let secret = std::env::var("JWT_SECRET")?;
    let exp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + PENDING_TOKEN_TTL_SECONDS;
    let claims = OtpClaims {
        token_type: PENDING_TOKEN_TYPE.to_string(),
        email: email.to_string(),
        username: username.to_string(),
        password_hash: password_hash.to_string(),
        otp_hash: otp_hash.to_string(),
        exp: exp as usize,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

/// Decode and validate a pending signup JWT.
/// Returns an error if the token is expired, tampered, or not a pending token.
pub fn decode_pending_token(
    token: &str,
) -> Result<OtpClaims, Box<dyn std::error::Error + Send + Sync>> {
    let secret = std::env::var("JWT_SECRET")?;
    let data = decode::<OtpClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    if data.claims.token_type != PENDING_TOKEN_TYPE {
        return Err("invalid token type".into());
    }
    Ok(data.claims)
}

/// Parse a single cookie value from a `Cookie: a=1; b=2` header string.
fn parse_cookie(cookie_header: &str, name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|pair| {
        let pair = pair.trim();
        let (k, v) = pair.split_once('=')?;
        (k.trim() == name).then(|| v.trim().to_owned())
    })
}

/// Append `Set-Cookie` headers for both tokens.
/// In debug builds `Secure` is omitted so plain-HTTP local testing works.
pub fn set_token_cookies(
    response: &mut Response,
    access_token: &str,
    refresh_token: &str,
) -> Result<(), header::InvalidHeaderValue> {
    #[cfg(debug_assertions)]
    let flags = "HttpOnly; SameSite=Strict";
    #[cfg(not(debug_assertions))]
    let flags = "HttpOnly; Secure; SameSite=Strict";

    let access = format!(
        "{COOKIE_ACCESS}={access_token}; {flags}; Max-Age={ACCESS_TOKEN_TTL_SECONDS}; Path=/"
    );
    let refresh = format!(
        "{COOKIE_REFRESH}={refresh_token}; {flags}; Max-Age={REFRESH_TOKEN_TTL_SECONDS}; Path=/"
    );

    response
        .headers_mut()
        .append(header::SET_COOKIE, HeaderValue::from_str(&access)?);
    response
        .headers_mut()
        .append(header::SET_COOKIE, HeaderValue::from_str(&refresh)?);
    Ok(())
}

/// Append `Set-Cookie` headers that instruct the browser to delete both cookies.
pub fn clear_token_cookies(response: &mut Response) {
    for name in [COOKIE_ACCESS, COOKIE_REFRESH] {
        let value = format!("{name}=; HttpOnly; SameSite=Strict; Max-Age=0; Path=/");
        response.headers_mut().append(
            header::SET_COOKIE,
            HeaderValue::from_str(&value).expect("static clear-cookie value is always valid"),
        );
    }
}

pub async fn jwt_auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    // Clone to String so we release the immutable borrow on `req` before mutating it.
    let cookie_str = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_owned();

    let access_token = parse_cookie(&cookie_str, COOKIE_ACCESS).ok_or(StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match decode::<Claims>(
        &access_token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) if data.claims.token_type == ACCESS_TOKEN_TYPE => {
            req.extensions_mut().insert(data.claims);
            Ok(next.run(req).await)
        }
        Ok(_) => Err(StatusCode::UNAUTHORIZED),
        Err(err) if matches!(err.kind(), ErrorKind::ExpiredSignature) => {
            refresh_access_token(req, next, &cookie_str, &secret).await
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn refresh_access_token(
    mut req: Request,
    next: Next,
    cookie_str: &str,
    secret: &str,
) -> Result<Response, StatusCode> {
    let refresh_token = parse_cookie(cookie_str, COOKIE_REFRESH).ok_or(StatusCode::UNAUTHORIZED)?;

    let data = decode::<Claims>(
        &refresh_token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if data.claims.token_type != REFRESH_TOKEN_TYPE {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user_id = data.claims.sub;
    let new_access = create_access_token(user_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let new_refresh =
        create_refresh_token(user_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    req.extensions_mut().insert(Claims {
        sub: user_id,
        exp: data.claims.exp,
        token_type: ACCESS_TOKEN_TYPE.to_string(),
    });

    let mut response = next.run(req).await;
    set_token_cookies(&mut response, &new_access, &new_refresh)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to set up the env var for tests
    fn setup_env() {
        unsafe {
            std::env::set_var("JWT_SECRET", "super-secret-test-key-12345");
        }
    }

    #[test]
    fn test_create_and_decode_access_token() {
        setup_env();
        let user_id = Uuid::new_v4();

        let token = create_access_token(user_id).expect("failed to create access token");
        assert!(!token.is_empty());

        let secret = std::env::var("JWT_SECRET").unwrap();
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .expect("failed to decode token");

        assert_eq!(decoded.claims.sub, user_id);
        assert_eq!(decoded.claims.token_type, ACCESS_TOKEN_TYPE);
    }

    #[test]
    fn test_create_and_decode_refresh_token() {
        setup_env();
        let user_id = Uuid::new_v4();

        let token = create_refresh_token(user_id).unwrap();

        let secret = std::env::var("JWT_SECRET").unwrap();
        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .unwrap();

        assert_eq!(decoded.claims.sub, user_id);
        assert_eq!(decoded.claims.token_type, REFRESH_TOKEN_TYPE);
    }

    #[test]
    fn test_pending_token_roundtrip() {
        setup_env();
        let email = "test@example.com";
        let username = "tester";
        let pass_hash = "hashed_pass";
        let otp_hash = "hashed_otp";

        let token = create_pending_token(email, username, pass_hash, otp_hash)
            .expect("failed to create pending token");

        let claims = decode_pending_token(&token).expect("failed to decode pending token");

        assert_eq!(claims.email, email);
        assert_eq!(claims.username, username);
        assert_eq!(claims.password_hash, pass_hash);
        assert_eq!(claims.otp_hash, otp_hash);
        assert_eq!(claims.token_type, PENDING_TOKEN_TYPE);
    }

    #[test]
    fn test_decode_pending_token_rejects_access_token() {
        setup_env();
        let user_id = Uuid::new_v4();
        let access_token = create_access_token(user_id).unwrap();

        // Trying to decode an access token as a pending token should fail
        let result = decode_pending_token(&access_token);
        assert!(result.is_err(), "Should have rejected access token");
    }

    #[test]
    fn test_parse_cookie_success() {
        let header = "access_token=123; refresh_token=456; other=789";
        assert_eq!(
            parse_cookie(header, "access_token"),
            Some("123".to_string())
        );
        assert_eq!(
            parse_cookie(header, "refresh_token"),
            Some("456".to_string())
        );
        assert_eq!(parse_cookie(header, "other"), Some("789".to_string()));
    }

    #[test]
    fn test_parse_cookie_missing() {
        let header = "access_token=123; other=789";
        assert_eq!(parse_cookie(header, "refresh_token"), None);
        assert_eq!(parse_cookie(header, "missing"), None);
    }

    #[test]
    fn test_parse_cookie_empty_string() {
        assert_eq!(parse_cookie("", "access_token"), None);
    }
}
