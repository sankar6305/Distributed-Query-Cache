use std::sync::Arc;
use std::time::Instant;
use tonic::{Request, Response, Status};

use crate::cache::Cache;
use crate::db::Database;

pub mod pb {
    tonic::include_proto!("query_cache");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("query_cache_descriptor");
}

use pb::{
    query_cache_server::QueryCache, HealthRequest, HealthResponse, QueryRequest, QueryResponse,
    ServingStatus, WarmUpRequest, WarmUpResponse,
};

pub struct QueryCacheSvc {
    pub db: Arc<Database>,
    pub cache: Arc<Cache>,
}

#[tonic::async_trait]
impl QueryCache for QueryCacheSvc {
    async fn run_query(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        let sql = request.into_inner().sql;
        let key = Cache::key_for(&sql);

        let start = Instant::now();

        // Cache lookup first — a hit skips DuckDB entirely.
        if let Some(cached_bytes) = self.cache.get(&key).await {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            let row_count = crate::flatbuf::decode(&cached_bytes)
                .map(|decoded| decoded.rows.len() as u64)
                .unwrap_or(0);

            return Ok(Response::new(QueryResponse {
                cache_hit: true,
                latency_ms,
                row_count,
                result: cached_bytes,
            }));
        }

        // Miss: run the real query, cache the encoded result, then return it.
        let result = self
            .db
            .run_query(sql)
            .await
            .map_err(Status::invalid_argument)?;

        let row_count = result.rows.len() as u64;
        let result_bytes = crate::flatbuf::encode(&result);

        self.cache.set(&key, &result_bytes).await;

        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(Response::new(QueryResponse {
            cache_hit: false,
            latency_ms,
            row_count,
            result: result_bytes,
        }))
    }

    async fn warm_up(
        &self,
        _request: Request<WarmUpRequest>,
    ) -> Result<Response<WarmUpResponse>, Status> {
        // Wired up in Phase 5.
        Err(Status::unimplemented("warm_up not implemented yet"))
    }

    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        let duckdb_ok = self.db.is_healthy();
        let redis_ok = self.cache.is_healthy().await;
        Ok(Response::new(HealthResponse {
            status: ServingStatus::Serving as i32,
            redis_ok,
            duckdb_ok,
        }))
    }
}
