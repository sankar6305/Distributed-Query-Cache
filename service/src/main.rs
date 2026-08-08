mod grpc;

use grpc::pb::query_cache_server::QueryCacheServer;
use grpc::pb::FILE_DESCRIPTOR_SET;
use grpc::QueryCacheSvc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;

    println!("query-cache-service listening on {addr}");

    tonic::transport::Server::builder()
        .add_service(QueryCacheServer::new(QueryCacheSvc))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
