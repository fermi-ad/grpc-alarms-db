use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let alarm_proto_dir: String =
        String::from("extern/proto-defs/proto/controls/service/grpc-alarms-db/v1");
    let incl: &[String] = &[alarm_proto_dir.clone()];
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let protoc_path = protoc_bin_vendored::protoc_bin_path()?;
    unsafe { std::env::set_var("PROTOC", protoc_path) };

    tonic_prost_build::configure()
        .build_client(false)
        .build_server(true)
        .protoc_arg("--experimental_allow_proto3_optional")
        .file_descriptor_set_path(out_dir.join("alarmprotos_descriptor.bin"))
        .compile_protos(
            &[
                format!("{}/alarm-groups.proto", alarm_proto_dir),
                format!("{}/user-layouts.proto", alarm_proto_dir),
            ],
            incl,
        )?;

    Ok(())
}
