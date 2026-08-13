mod cache;
mod db;
mod flatbuf;
mod grpc;
mod schema;

use std::env;
use std::sync::Arc;
use std::time::Duration;

use cache::Cache;
use db::Database;
use grpc::pb::query_cache_server::QueryCacheServer;
use grpc::pb::FILE_DESCRIPTOR_SET;
use grpc::QueryCacheSvc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;

    println!("seeding embedded DuckDB (2M synthetic rows)...");
    let db = Arc::new(Database::open_seeded()?);
    println!("DuckDB ready.");

    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".into());
    let ttl_secs: u64 = env::var("CACHE_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    println!("connecting to Redis at {redis_url} (TTL {ttl_secs}s)...");
    let cache = Arc::new(Cache::connect(&redis_url, Duration::from_secs(ttl_secs)).await?);
    println!("Redis ready.");

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    let service = QueryCacheSvc { db, cache };

    println!("query-cache-service listening on {addr}");

    tonic::transport::Server::builder()
        .add_service(QueryCacheServer::new(service))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
