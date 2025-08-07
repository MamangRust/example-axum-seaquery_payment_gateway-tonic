use redis::{Commands, Connection};
use serde::{Serialize, de::DeserializeOwned};
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, warn};

#[derive(Clone)]
pub struct CacheStore {
    pub redis: Arc<redis::Client>,
}

impl CacheStore {
    pub fn new(redis: redis::Client) -> Self {
        Self {
            redis: Arc::new(redis),
        }
    }

    fn get_conn(&self) -> Option<Connection> {
        match self.redis.get_connection() {
            Ok(conn) => Some(conn),
            Err(e) => {
                error!("Failed to get Redis connection: {:?}", e);
                None
            }
        }
    }

    pub fn get_from_cache<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.get_conn()?;
        let result: redis::RedisResult<Option<String>> = conn.get(key);

        match result {
            Ok(Some(data)) => match serde_json::from_str::<T>(&data) {
                Ok(parsed) => Some(parsed),
                Err(e) => {
                    error!("Failed to deserialize cached value: {:?}", e);
                    None
                }
            },
            Ok(None) => {
                warn!("Cache miss for key: {key}");
                None
            }
            Err(e) => {
                error!("Redis get error for key {e}: {:?}", key);
                None
            }
        }
    }

    pub fn set_to_cache<T>(&self, key: &str, data: &T, expiration: Duration)
    where
        T: Serialize,
    {
        let json_data = match serde_json::to_string(data) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize data: {:?}", e);
                return;
            }
        };

        let conn = self.get_conn();
        if let Some(mut conn) = conn {
            let result: redis::RedisResult<()> = redis::pipe()
                .cmd("SET")
                .arg(key)
                .arg(&json_data)
                .ignore()
                .cmd("EXPIRE")
                .arg(key)
                .arg(expiration.as_secs() as usize)
                .query(&mut conn);

            match result {
                Ok(_) => debug!("Cached key {} with TTL {:?}", key, expiration),
                Err(e) => error!("Failed to set cache key {e}: {:?}", key),
            }
        }
    }

    pub fn delete_from_cache(&self, key: &str) {
        if let Some(mut conn) = self.get_conn()
            && let Err(e) = redis::cmd("DEL").arg(key).query::<()>(&mut conn)
        {
            error!("Failed to delete key {e}: {:?}", key);
        }
    }
}
