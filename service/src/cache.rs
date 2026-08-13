use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub struct Cache {
    conn: ConnectionManager,
    ttl_secs: u64,
}

impl Cache {
    /// `ConnectionManager` holds a single multiplexed connection and
    /// reconnects automatically on failure; it's cheap to `.clone()` (an
    /// `Arc` under the hood), so every request just clones it instead of
    /// checking a connection out of a pool.
    pub async fn connect(redis_url: &str, ttl: Duration) -> redis::RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_connection_manager().await?;
        Ok(Self {
            conn,
            ttl_secs: ttl.as_secs().max(1),
        })
    }

    /// Cache key = a hash of the normalized SQL text. Not cryptographic —
    /// collisions just mean two different queries would (extremely rarely)
    /// share a cache slot, which is an acceptable trade for speed here.
    pub fn key_for(sql: &str) -> String {
        let mut hasher = DefaultHasher::new();
        sql.trim().hash(&mut hasher);
        format!("qc:query:{:016x}", hasher.finish())
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut conn = self.conn.clone();
        conn.get::<_, Option<Vec<u8>>>(key).await.ok().flatten()
    }

    pub async fn set(&self, key: &str, value: &[u8]) {
        let mut conn = self.conn.clone();
        // Best-effort: if Redis is down, we fail open to DuckDB rather than
        // erroring the request out.
        let _: redis::RedisResult<()> = conn.set_ex(key, value, self.ttl_secs).await;
    }

    pub async fn is_healthy(&self) -> bool {
        let mut conn = self.conn.clone();
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .is_ok()
    }
}
