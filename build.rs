use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const ALARM_PROTO_DIR: &str = "extern/proto-defs/proto/controls/service/grpc-alarms-db/v1";
    let incl: &[&str] = &[ALARM_PROTO_DIR];
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    unsafe { std::env::set_var("PROTOC", protoc_path) };

    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .file_descriptor_set_path(out_dir.join("alarmprotos_descriptor.bin"))
        .compile_protos(
            &[
                format!("{}/alarm-groups.proto", ALARM_PROTO_DIR),
                format!("{}/user-layouts.proto", ALARM_PROTO_DIR),
            ],
            incl,
        )?;

    Ok(())
}
