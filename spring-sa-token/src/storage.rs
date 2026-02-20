//! Spring Redis Storage for Sa-Token
//!
//! This module provides a storage implementation that reuses the Redis connection
//! from `spring-redis` plugin, avoiding duplicate connections.

use sa_token_adapter::storage::{SaStorage, StorageError, StorageResult};
use spring::async_trait;
use spring_redis::redis::AsyncCommands;
use spring_redis::Redis;
use std::time::Duration;

/// Redis storage implementation using spring-redis connection
///
/// This storage reuses the `Redis` (ConnectionManager) component from `spring-redis`,
/// so you don't need to configure a separate Redis connection for sa-token.
pub struct SpringRedisStorage {
    client: Redis,
}

impl SpringRedisStorage {
    /// Create a new SpringRedisStorage with the given Redis connection
    pub fn new(client: Redis) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SaStorage for SpringRedisStorage {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        let mut conn = self.client.clone();
        tracing::debug!("SpringRedisStorage GET key: {}", key);
        conn.get(key)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()> {
        let mut conn = self.client.clone();
        tracing::debug!("SpringRedisStorage SET key: {}", key);

        if let Some(ttl) = ttl {
            conn.set_ex(key, value, ttl.as_secs())
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))
        } else {
            conn.set(key, value)
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))
        }
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut conn = self.client.clone();

        conn.del(key)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let mut conn = self.client.clone();

        conn.exists(key)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()> {
        let mut conn = self.client.clone();

        conn.expire(key, ttl.as_secs() as i64)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>> {
        let mut conn = self.client.clone();

        let ttl_secs: i64 = conn
            .ttl(key)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        match ttl_secs {
            -2 => Ok(None), // Key does not exist
            -1 => Ok(None), // Key exists but has no expiry
            secs if secs > 0 => Ok(Some(Duration::from_secs(secs as u64))),
            _ => Ok(Some(Duration::from_secs(0))),
        }
    }

    async fn mget(&self, keys: &[&str]) -> StorageResult<Vec<Option<String>>> {
        let mut conn = self.client.clone();

        // Use mget command for multiple keys
        spring_redis::redis::cmd("MGET")
            .arg(keys)
            .query_async(&mut conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn mset(&self, items: &[(&str, &str)], ttl: Option<Duration>) -> StorageResult<()> {
        let mut conn = self.client.clone();

        // Use pipeline for batch operations
        let mut pipe = spring_redis::redis::pipe();
        for (key, value) in items {
            if let Some(ttl) = ttl {
                pipe.set_ex(*key, *value, ttl.as_secs());
            } else {
                pipe.set(*key, *value);
            }
        }

        pipe.query_async(&mut conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn mdel(&self, keys: &[&str]) -> StorageResult<()> {
        let mut conn = self.client.clone();

        conn.del(keys)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn incr(&self, key: &str) -> StorageResult<i64> {
        let mut conn = self.client.clone();

        conn.incr(key, 1)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn decr(&self, key: &str) -> StorageResult<i64> {
        let mut conn = self.client.clone();

        conn.decr(key, 1)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }

    async fn clear(&self) -> StorageResult<()> {
        let mut conn = self.client.clone();
        let pattern = "sa:*";

        // Get all matching keys
        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        if !keys.is_empty() {
            conn.del::<_, ()>(&keys)
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
        }

        Ok(())
    }

    async fn keys(&self, pattern: &str) -> StorageResult<Vec<String>> {
        let mut conn = self.client.clone();

        conn.keys(pattern)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))
    }
}