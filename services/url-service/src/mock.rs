#![cfg(test)]
use crate::models::Url;
use crate::repo_trait::UrlRepo;
use crate::repository::UrlsPage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicI64, Ordering};
use uuid::Uuid;

pub struct MockUrlRepo {
    urls: RwLock<HashMap<i64, Url>>,
    next_id: AtomicI64,
}

impl MockUrlRepo {
    pub fn new() -> Self {
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

        if let Some(existing) = urls
            .values()
            .find(|u| u.original_url == original_url && u.user_id == user_id)
        {
            if let Some(exp) = existing.expires_at
                && exp < Utc::now()
            {
                let mut updated = existing.clone();
                updated.expires_at = expires_at;
                urls.insert(updated.url_id, updated.clone());
                return Ok(Some(updated));
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

    async fn delete_url(&self, url_id: i64, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let mut urls = self.urls.write().unwrap();
        if let Some(url) = urls.get(&url_id)
            && url.user_id == user_id
        {
            let code = url.short_code.clone();
            urls.remove(&url_id);
            return Ok(Some(code));
        }
        Ok(None)
    }

    async fn ping(&self) -> Result<(), sqlx::Error> {
        Ok(())
    }
}
