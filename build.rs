fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the PROTOC environment variable to use vendored protoc
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    // Use tonic-build to compile protobuf
    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &[
                "protos/geyser.proto",
            ],
            &["protos"],
        )?;
    Ok(())
}
