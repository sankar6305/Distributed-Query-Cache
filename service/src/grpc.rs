use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("query_cache");

    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("query_cache_descriptor");
}

use pb::{
    query_cache_server::QueryCache, HealthRequest, HealthResponse, QueryRequest, QueryResponse,
    ServingStatus, WarmUpRequest, WarmUpResponse,
};

#[derive(Default)]
pub struct QueryCacheSvc;

#[tonic::async_trait]
impl QueryCache for QueryCacheSvc {
    async fn run_query(
        &self,
        _request: Request<QueryRequest>,
    ) -> Result<Response<QueryResponse>, Status> {
        // Wired up in Phase 2 (DuckDB) and Phase 4 (Redis cache lookup).
        Err(Status::unimplemented("run_query not implemented yet"))
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
        // Real Redis/DuckDB connectivity checks land in Phase 5.
        Ok(Response::new(HealthResponse {
            status: ServingStatus::Serving as i32,
            redis_ok: true,
            duckdb_ok: true,
        }))
    }
}
