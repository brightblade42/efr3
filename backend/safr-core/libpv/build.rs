fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    println!("cargo:rerun-if-changed=proto/proc/processor.proto");
    println!("cargo:rerun-if-changed=proto/proc/processor_service.proto");
    println!("cargo:rerun-if-changed=proto/proc/health_service.proto");
    println!("cargo:rerun-if-changed=proto/identity/identity_service.proto");
    println!("cargo:rerun-if-changed=proto/identity/models.proto");

    tonic_build::configure()
        .build_server(false)
        .compile_protos(
            &[
                "proto/proc/processor_service.proto",
                "proto/proc/health_service.proto",
                "proto/identity/identity_service.proto",
            ],
            &["proto/proc", "proto/identity"],
        )?;

    Ok(())
}
