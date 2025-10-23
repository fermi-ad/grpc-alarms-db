use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let incl: &[&str] = &["src/proto"];
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let protoc_path  = protoc_bin_vendored::protoc_bin_path()?;
    unsafe { std::env::set_var("PROTOC", protoc_path) };

    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .file_descriptor_set_path(out_dir.join("alarmprotos_descriptor.bin"))
        .compile_protos(&["src/proto/lists.proto"], incl)?;

    Ok(())
}