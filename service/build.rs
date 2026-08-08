fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR")?;

    tonic_prost_build::configure()
        .file_descriptor_set_path(std::path::Path::new(&out_dir).join("query_cache_descriptor.bin"))
        .compile_protos(&["../proto/query_cache.proto"], &["../proto"])?;

    Ok(())
}
