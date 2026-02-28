//! SeaORM-based storage implementation for Sa-Token.
//!
//! This module provides a custom `SaStorage` implementation using SeaORM,
//! demonstrating how users can define their own storage backend.

use async_trait::async_trait;
use summer::plugin::service::Service;
use summer_sa_token::sa_token_adapter::storage::{SaStorage, StorageError, StorageResult};
use sea_orm::entity::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use summer_sea_orm::DbConn;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Simple key/value storage table for Sa-Token.
///
/// SQL:
/// ```sql
/// CREATE TABLE sa_token_storage (
///   key TEXT PRIMARY KEY,
///   value TEXT NOT NULL,
///   expire_at BIGINT NULL
/// );
/// ```
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sa_token_storage")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub key: String,
    pub value: String,
    /// Unix timestamp (seconds) when this entry expires; NULL means no expiry.
    pub expire_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}


/// SeaORM-based storage for Sa-Token.
///
/// Uses `#[derive(Service)]` to automatically inject `DbConn` from the component registry.
/// This is the recommended way to define custom storage - no need to manually handle
/// `LazyComponent` or other framework internals.
///
/// # Example
///
/// ```rust,ignore
/// use summer_sa_token::{lazy_storage, SaTokenConfigurator};
///
/// impl SaTokenConfigurator for MyConfig {
///     fn configure_storage(&self, _app: &AppBuilder) -> Option<Arc<dyn SaStorage>> {
///         Some(lazy_storage::<SeaOrmStorage>())
///     }
/// }
/// ```
#[derive(Clone, Service)]
pub struct SeaOrmStorage {
    #[inject(component)]
    conn: DbConn,
}

impl SeaOrmStorage {
    /// Get current Unix timestamp in seconds.
    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Calculate expire_at from TTL.
    fn calc_expire_at(ttl: Option<Duration>) -> Option<i64> {
        ttl.map(|d| Self::now_secs() + d.as_secs() as i64)
    }

    /// Check if a model is expired.
    fn is_expired(model: &Model) -> bool {
        model
            .expire_at
            .map(|exp| exp <= Self::now_secs())
            .unwrap_or(false)
    }
}

#[async_trait]
impl SaStorage for SeaOrmStorage {
    async fn get(&self, key: &str) -> StorageResult<Option<String>> {
        let result = Entity::find_by_id(key.to_string())
            .one(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        match result {
            Some(model) if !Self::is_expired(&model) => Ok(Some(model.value)),
            Some(_) => {
                // Expired - delete and return None
                let _ = self.delete(key).await;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> StorageResult<()> {
        let expire_at = Self::calc_expire_at(ttl);

        let model = ActiveModel {
            key: Set(key.to_string()),
            value: Set(value.to_string()),
            expire_at: Set(expire_at),
        };

        Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(Column::Key)
                    .update_columns([Column::Value, Column::ExpireAt])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        Entity::delete_by_id(key.to_string())
            .exec(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let result = Entity::find_by_id(key.to_string())
            .one(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        match result {
            Some(model) if !Self::is_expired(&model) => Ok(true),
            Some(_) => {
                // Expired - delete and return false
                let _ = self.delete(key).await;
                Ok(false)
            }
            None => Ok(false),
        }
    }

    async fn expire(&self, key: &str, ttl: Duration) -> StorageResult<()> {
        let expire_at = Self::calc_expire_at(Some(ttl));

        Entity::update_many()
            .col_expr(Column::ExpireAt, Expr::value(expire_at))
            .filter(Column::Key.eq(key))
            .exec(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn ttl(&self, key: &str) -> StorageResult<Option<Duration>> {
        let result = Entity::find_by_id(key.to_string())
            .one(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        match result {
            Some(model) => match model.expire_at {
                Some(exp) => {
                    let now = Self::now_secs();
                    if exp <= now {
                        Ok(None) // Expired
                    } else {
                        Ok(Some(Duration::from_secs((exp - now) as u64)))
                    }
                }
                None => Ok(None), // No expiry set
            },
            None => Ok(None), // Key not found
        }
    }

    async fn mget(&self, keys: &[&str]) -> StorageResult<Vec<Option<String>>> {
        let key_strings: Vec<String> = keys.iter().map(|s| String::from(*s)).collect();
        let results = Entity::find()
            .filter(Column::Key.is_in(key_strings.clone()))
            .all(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        // Build a map for quick lookup
        let map: std::collections::HashMap<_, _> =
            results.into_iter().map(|m| (m.key.clone(), m)).collect();

        // Return in original order
        let mut output = Vec::with_capacity(keys.len());
        for key in keys {
            match map.get(*key) {
                Some(model) if !Self::is_expired(model) => {
                    output.push(Some(model.value.clone()));
                }
                _ => output.push(None),
            }
        }

        Ok(output)
    }

    async fn mset(&self, items: &[(&str, &str)], ttl: Option<Duration>) -> StorageResult<()> {
        let expire_at = Self::calc_expire_at(ttl);

        for (key, value) in items {
            let model = ActiveModel {
                key: Set(String::from(*key)),
                value: Set(String::from(*value)),
                expire_at: Set(expire_at),
            };

            Entity::insert(model)
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column(Column::Key)
                        .update_columns([Column::Value, Column::ExpireAt])
                        .to_owned(),
                )
                .exec(&self.conn)
                .await
                .map_err(|e| StorageError::OperationFailed(e.to_string()))?;
        }

        Ok(())
    }

    async fn mdel(&self, keys: &[&str]) -> StorageResult<()> {
        let key_strings: Vec<String> = keys.iter().map(|s| String::from(*s)).collect();

        Entity::delete_many()
            .filter(Column::Key.is_in(key_strings))
            .exec(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn incr(&self, key: &str) -> StorageResult<i64> {
        // Get current value, parse as i64, increment, and set back
        let current = self.get(key).await?;
        let value: i64 = current
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let new_value = value + 1;

        self.set(key, &new_value.to_string(), None).await?;
        Ok(new_value)
    }

    async fn decr(&self, key: &str) -> StorageResult<i64> {
        let current = self.get(key).await?;
        let value: i64 = current
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let new_value = value - 1;

        self.set(key, &new_value.to_string(), None).await?;
        Ok(new_value)
    }

    async fn clear(&self) -> StorageResult<()> {
        // Delete all rows with keys starting with "sa:"
        Entity::delete_many()
            .filter(Column::Key.starts_with("sa:"))
            .exec(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn keys(&self, pattern: &str) -> StorageResult<Vec<String>> {
        // Convert simple glob pattern to SQL LIKE pattern
        let like_pattern = pattern.replace('*', "%").replace('?', "_");

        let results = Entity::find()
            .filter(Column::Key.like(&like_pattern))
            .all(&self.conn)
            .await
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        // Filter out expired keys
        let now = Self::now_secs();
        let keys: Vec<String> = results
            .into_iter()
            .filter(|m| m.expire_at.map(|exp| exp > now).unwrap_or(true))
            .map(|m| m.key)
            .collect();

        Ok(keys)
    }
}