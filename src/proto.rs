//! Module to bring generated protobuf artifacts into the dependency tree.

include!(concat!(env!("OUT_DIR"), "/proto.rs"));
