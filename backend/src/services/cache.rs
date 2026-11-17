//! Per-user cache for `GET /tasks/view-my-tasks`.
//!
//! `Cache` is a small async trait with two implementations:
//! * `RedisCache` - default when `REDIS_URL` is set (shared across API instances,
//!   survives API restarts).
//! * `InMemoryCache` - fallback for tests / when Redis is unavailable. Limitation:
//!   process-local, lost on restart, not shared between multiple API replicas.
//!
//! Cache failures never fail a request: reads degrade to a DB query and the
//! error is logged.

use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, thiserror::Error)]
#[error("cache error: {0}")]
pub struct CacheError(String);

impl From<redis::RedisError> for CacheError {
    fn from(e: redis::RedisError) -> Self {
        Self(e.to_string())
    }
}

#[async_trait]
pub trait Cache: Send + Sync {
    fn backend_name(&self) -> &'static str;
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;
    async fn set(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError>;
    async fn delete(&self, keys: &[String]) -> Result<(), CacheError>;
    async fn ping(&self) -> Result<(), CacheError>;
}

pub fn my_tasks_key(user_id: Uuid) -> String {
    format!("taskmgr:tasks:my:{user_id}")
}

/// Drops the cached task list for each affected user. Errors are logged; the
/// TTL bounds staleness if an invalidation is ever missed.
pub async fn invalidate_my_tasks(cache: &dyn Cache, user_ids: impl IntoIterator<Item = Uuid>) {
    let keys: Vec<String> = user_ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .map(my_tasks_key)
        .collect();
    if keys.is_empty() {
        return;
    }
    match cache.delete(&keys).await {
        Ok(()) => tracing::debug!(?keys, "invalidated my-tasks cache"),
        Err(e) => tracing::error!(error = %e, ?keys, "failed to invalidate my-tasks cache"),
    }
}

// ---------------- Redis ----------------

pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    pub async fn connect(url: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Cache for RedisCache {
    fn backend_name(&self) -> &'static str {
        "redis"
    }

    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let mut conn = self.conn.clone();
        Ok(conn.get(key).await?)
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        let _: () = conn.set_ex(key, value, ttl.as_secs().max(1)).await?;
        Ok(())
    }

    async fn delete(&self, keys: &[String]) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        let _: () = conn.del(keys).await?;
        Ok(())
    }

    async fn ping(&self) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }
}

// ---------------- In-memory ----------------

pub struct InMemoryCache {
    inner: moka::future::Cache<String, (String, Instant)>,
}

impl InMemoryCache {
    pub fn new(max_entries: u64) -> Self {
        Self {
            inner: moka::future::Cache::new(max_entries),
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    fn backend_name(&self) -> &'static str {
        "in-memory"
    }

    async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        match self.inner.get(key).await {
            Some((value, expires_at)) if Instant::now() < expires_at => Ok(Some(value)),
            Some(_) => {
                self.inner.invalidate(key).await;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: String, ttl: Duration) -> Result<(), CacheError> {
        self.inner
            .insert(key.to_string(), (value, Instant::now() + ttl))
            .await;
        Ok(())
    }

    async fn delete(&self, keys: &[String]) -> Result<(), CacheError> {
        for key in keys {
            self.inner.invalidate(key).await;
        }
        Ok(())
    }

    async fn ping(&self) -> Result<(), CacheError> {
        Ok(())
    }
}

/// Redis when configured and reachable, otherwise in-memory (with a warning).
pub async fn build_cache(config: &Config) -> Arc<dyn Cache> {
    if let Some(url) = &config.redis_url {
        match RedisCache::connect(url).await {
            Ok(cache) => match cache.ping().await {
                Ok(()) => {
                    tracing::info!("cache backend: redis");
                    return Arc::new(cache);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "redis ping failed, falling back to in-memory cache")
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "redis unavailable, falling back to in-memory cache")
            }
        }
    } else {
        tracing::info!("REDIS_URL not set");
    }
    tracing::warn!("cache backend: in-memory (process-local, not shared between replicas)");
    Arc::new(InMemoryCache::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_set_get_delete_and_expiry() {
        let cache = InMemoryCache::default();
        cache
            .set("k", "v".into(), Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.get("k").await.unwrap().as_deref(), Some("v"));
        cache.delete(&["k".to_string()]).await.unwrap();
        assert_eq!(cache.get("k").await.unwrap(), None);

        cache
            .set("e", "v".into(), Duration::from_millis(10))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(cache.get("e").await.unwrap(), None);
    }
}
