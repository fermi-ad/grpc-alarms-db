//! Utilities Module
//!
//! Provides utility functions that have application-wide applicability.

use crate::proto::google::protobuf::Timestamp;
use chrono::{DateTime, Utc};

/// Converts a `DateTime<Utc>` to a Protobuf `Timestamp`.
/// Wraps the result in `Some` for convenience -> Complex types in Protobuf are often represented as `Option<T>` in Rust.
pub fn datetime_to_timestamp(datetime: DateTime<Utc>) -> Option<Timestamp> {
    Some(Timestamp {
        seconds: datetime.timestamp(),
        nanos: datetime.timestamp_subsec_nanos() as i32,
    })
}
