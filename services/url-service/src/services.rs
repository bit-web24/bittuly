use crate::{models::Url, repo_trait::UrlRepo, repository::UrlsPage};
use uuid::Uuid;

pub async fn shorten_url(
    repo: &dyn UrlRepo,
    original_url: &str,
    user_id: Uuid,
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Option<Url>, sqlx::Error> {
    repo.add_shorten_url(original_url, user_id, expires_at)
        .await
}

pub async fn get_original_url(
    repo: &dyn UrlRepo,
    short_code: &str,
) -> Result<Option<(String, Option<chrono::DateTime<chrono::Utc>>)>, sqlx::Error> {
    repo.get_original_url(short_code).await
}

pub async fn get_urls_page(
    repo: &dyn UrlRepo,
    user_id: Uuid,
    cursor: Option<i64>,
    limit: i64,
    search: Option<String>,
) -> Result<UrlsPage, sqlx::Error> {
    repo.get_urls_page(user_id, cursor, limit, search).await
}

/// Returns `Some(short_code)` if deleted, `None` if not found or not owned by the user.
pub async fn delete_url(
    repo: &dyn UrlRepo,
    url_id: i64,
    user_id: Uuid,
) -> Result<Option<String>, sqlx::Error> {
    repo.delete_url(url_id, user_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockUrlRepo;
    use chrono::{DateTime, Utc};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_shorten_new_url() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        let url = shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(url.original_url, "https://google.com");
        assert_eq!(url.short_code, "1"); // base62 of 1
    }

    #[tokio::test]
    async fn test_shorten_duplicate_url() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap()
            .unwrap();
        let duplicate = shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap();
        assert!(duplicate.is_none());
    }

    #[tokio::test]
    async fn test_shorten_expired_url_reactivation() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        let past = Utc::now() - chrono::Duration::days(1);
        shorten_url(&repo, "https://google.com", user_id, Some(past))
            .await
            .unwrap()
            .unwrap();

        let future = Utc::now() + chrono::Duration::days(1);
        let reactivated = shorten_url(&repo, "https://google.com", user_id, Some(future))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(reactivated.expires_at, Some(future));
    }

    #[tokio::test]
    async fn test_get_original_url() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        let url = shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap()
            .unwrap();

        let found = get_original_url(&repo, &url.short_code)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.0, "https://google.com");
    }

    #[tokio::test]
    async fn test_delete_url_success() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        let url = shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap()
            .unwrap();

        let deleted = delete_url(&repo, url.url_id, user_id).await.unwrap();
        assert_eq!(deleted, Some(url.short_code));
    }

    #[tokio::test]
    async fn test_delete_url_wrong_user() {
        let repo = MockUrlRepo::new();
        let user_id = Uuid::new_v4();
        let wrong_user = Uuid::new_v4();
        let url = shorten_url(&repo, "https://google.com", user_id, None)
            .await
            .unwrap()
            .unwrap();

        let deleted = delete_url(&repo, url.url_id, wrong_user).await.unwrap();
        assert_eq!(deleted, None);
    }
}
