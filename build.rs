//! Build script for this application.
//! Sets up the gRPC interfaces and constructs the Rust implementations of the
//! gRPC message objects.

use rust_grpc_lib::build_support::{Config, generate_protos};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_protos(Config::new())
}
