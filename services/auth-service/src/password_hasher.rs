use async_trait::async_trait;

type HasherResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// ---------------------------------------------------------------------------
// Strategy trait
// ---------------------------------------------------------------------------

/// Abstracts password hashing so services never care about algorithm or cost.
///
/// Implementations decide *how* to hash — the service just calls `.hash()` /
/// `.verify()` and stays environment-agnostic.
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, plain: &str) -> HasherResult<String>;
    async fn verify(&self, plain: &str, hash: &str) -> HasherResult<bool>;
}

// ---------------------------------------------------------------------------
// Concrete strategy — bcrypt
// ---------------------------------------------------------------------------

pub struct BcryptHasher {
    pub cost: u32,
}

impl BcryptHasher {
    /// Choose cost based on the runtime mode:
    /// - `development` → 4   (minimum; keeps tests and local load-tests fast)
    /// - anything else → 10  (production; meets 400 ms SLO under moderate load)
    pub fn from_mode(mode: &str) -> Self {
        let cost = if mode == "development" { 4 } else { 10 };
        Self { cost }
    }
}

#[async_trait]
impl PasswordHasher for BcryptHasher {
    async fn hash(&self, plain: &str) -> HasherResult<String> {
        let plain = plain.to_string();
        let cost = self.cost;
        tokio::task::spawn_blocking(move || bcrypt::hash(&plain, cost))
            .await
            .map_err(|_| "task panicked")?
            .map_err(Into::into)
    }

    async fn verify(&self, plain: &str, hash: &str) -> HasherResult<bool> {
        let plain = plain.to_string();
        let hash = hash.to_string();
        tokio::task::spawn_blocking(move || bcrypt::verify(&plain, &hash))
            .await
            .map_err(|_| "task panicked")?
            .map_err(Into::into)
    }
}

// ---------------------------------------------------------------------------
// Test double (Null Object / Mock strategy)
// ---------------------------------------------------------------------------

/// A no-cost hasher for unit tests: stores the plain text as-is so tests
/// don't pay bcrypt overhead, and `verify` is a simple equality check.
pub struct PlainTextHasher;

#[async_trait]
impl PasswordHasher for PlainTextHasher {
    async fn hash(&self, plain: &str) -> HasherResult<String> {
        Ok(plain.to_string())
    }

    async fn verify(&self, plain: &str, hash: &str) -> HasherResult<bool> {
        Ok(plain == hash)
    }
}
