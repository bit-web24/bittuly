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
const ACCESS_TOKEN_TTL_SECONDS: u64 = 60 * 60 * 24; // 24 hours
const REFRESH_TOKEN_TTL_SECONDS: u64 = 60 * 60 * 24 * 30; // 30 days

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
    let expires_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + ttl_seconds;
    let claims = Claims {
        sub: user_id,
        exp: expires_at as usize,
        token_type: token_type.to_string(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
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

pub async fn jwt_auth(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let secret = std::env::var("JWT_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(token_data) if token_data.claims.token_type == ACCESS_TOKEN_TYPE => {
            req.extensions_mut().insert(token_data.claims);
            Ok(next.run(req).await)
        }
        Ok(_) => Err(StatusCode::UNAUTHORIZED),
        Err(err) if matches!(err.kind(), ErrorKind::ExpiredSignature) => {
            refresh_access_token(req, next, &secret).await
        }
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn refresh_access_token(
    mut req: Request,
    next: Next,
    secret: &str,
) -> Result<Response, StatusCode> {
    let refresh_token = req
        .headers()
        .get("x-refresh-token")
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let refresh_token_data = decode::<Claims>(
        refresh_token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if refresh_token_data.claims.token_type != REFRESH_TOKEN_TYPE {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user_id = refresh_token_data.claims.sub;
    let access_token =
        create_access_token(user_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let refresh_token =
        create_refresh_token(user_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    req.extensions_mut().insert(Claims {
        sub: user_id,
        exp: refresh_token_data.claims.exp,
        token_type: ACCESS_TOKEN_TYPE.to_string(),
    });

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        "x-access-token",
        HeaderValue::from_str(&access_token).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    response.headers_mut().insert(
        "x-refresh-token",
        HeaderValue::from_str(&refresh_token).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}
