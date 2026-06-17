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
    use async_trait::async_trait;
    use chrono::{DateTime, Utc};
    use std::collections::HashMap;
    use std::sync::{
        RwLock,
        atomic::{AtomicI64, Ordering},
    };

    struct MockUrlRepo {
        urls: RwLock<HashMap<i64, Url>>,
        next_id: AtomicI64,
    }

    impl MockUrlRepo {
        fn new() -> Self {
            Self {
                urls: RwLock::new(HashMap::new()),
                next_id: AtomicI64::new(1),
            }
        }
    }

    #[async_trait]
    impl UrlRepo for MockUrlRepo {
        async fn add_shorten_url(
            &self,
            original_url: &str,
            user_id: Uuid,
            expires_at: Option<DateTime<Utc>>,
        ) -> Result<Option<Url>, sqlx::Error> {
            let mut urls = self.urls.write().unwrap();

            // Check existing
            if let Some(existing) = urls
                .values()
                .find(|u| u.original_url == original_url && u.user_id == user_id)
            {
                if let Some(exp) = existing.expires_at {
                    if exp < Utc::now() {
                        let mut updated = existing.clone();
                        updated.expires_at = expires_at;
                        urls.insert(updated.url_id, updated.clone());
                        return Ok(Some(updated));
                    }
                }
                return Ok(None);
            }

            let id = self.next_id.fetch_add(1, Ordering::SeqCst);
            let url = Url {
                url_id: id,
                short_code: base62::encode(id as u128),
                original_url: original_url.to_string(),
                user_id,
                click_count: 0,
                expires_at,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            urls.insert(id, url.clone());
            Ok(Some(url))
        }

        async fn get_original_url(
            &self,
            short_code: &str,
        ) -> Result<Option<(String, Option<DateTime<Utc>>)>, sqlx::Error> {
            let urls = self.urls.read().unwrap();
            if let Some(url) = urls.values().find(|u| u.short_code == short_code) {
                Ok(Some((url.original_url.clone(), url.expires_at)))
            } else {
                Ok(None)
            }
        }

        async fn get_urls_page(
            &self,
            user_id: Uuid,
            cursor: Option<i64>,
            limit: i64,
            _search: Option<String>,
        ) -> Result<UrlsPage, sqlx::Error> {
            let urls = self.urls.read().unwrap();
            let mut user_urls: Vec<Url> = urls
                .values()
                .filter(|u| u.user_id == user_id)
                .cloned()
                .collect();

            user_urls.sort_by_key(|u| std::cmp::Reverse(u.url_id));

            let mut filtered = match cursor {
                Some(c) => user_urls
                    .into_iter()
                    .filter(|u| u.url_id < c)
                    .collect::<Vec<_>>(),
                None => user_urls,
            };

            let has_next = filtered.len() as i64 > limit;
            filtered.truncate(limit as usize);

            let next_cursor = if has_next {
                filtered.last().map(|u| format!("{:x}", u.url_id))
            } else {
                None
            };

            Ok(UrlsPage {
                urls: filtered,
                next_cursor,
            })
        }

        async fn delete_url(
            &self,
            url_id: i64,
            user_id: Uuid,
        ) -> Result<Option<String>, sqlx::Error> {
            let mut urls = self.urls.write().unwrap();
            if let Some(url) = urls.get(&url_id) {
                if url.user_id == user_id {
                    let code = url.short_code.clone();
                    urls.remove(&url_id);
                    return Ok(Some(code));
                }
            }
            Ok(None)
        }

        async fn ping(&self) -> Result<(), sqlx::Error> {
            Ok(())
        }
    }

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
