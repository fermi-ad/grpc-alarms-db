//! Alarm Timers Module
//!
//! Interacts with the database to store and retrieve data related to alarm timers (snooze and bypass reminders).

use crate::proto::google::protobuf::{Empty, Timestamp};
use crate::proto::services::alarm_timers::{
    AlarmTimer, AlarmTimers, DeleteRequest, ReadRequest, TimerType,
    alarm_timer_service_server::AlarmTimerService,
};
use crate::utils;
use chrono::{DateTime, Utc};
use queries::{
    CREATE_TIMER_QUERY, DELETE_TIMER_QUERY, READ_SNOOZE_TIMERS, READ_USER_BYPASS_REMINDERS_QUERY,
    UPDATE_TIMER_QUERY,
};
use rust_db_lib::{
    DataRow, DataStore, DataStoreError, DataVal, ParameterizedQuery, QueryParameter,
};
use std::marker::PhantomData;
use tonic::{Request, Response, Status};
use tracing::{error, info};

mod queries;

#[cfg(test)]
mod tests;

/// A service wrapping a [`DataStore`] to provide alarm timer information, and implementing the Protobuf-defined gRPC service.
pub struct AlarmTimersServiceImpl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> {
    data_store: V,
    _row_type: PhantomData<U>,
    _val_type: PhantomData<T>,
}
impl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> AlarmTimersServiceImpl<T, U, V> {
    pub fn new(data_store: V) -> Self {
        Self {
            data_store,
            _row_type: PhantomData,
            _val_type: PhantomData,
        }
    }

    async fn create_impl(&self, timer: ValidTimerInput) -> Result<(), DataStoreError> {
        info!(
            "Setting alarm timer for device: {}, type: {}",
            timer.device,
            timer.timer_type.as_string_name()
        );

        let mut query_builder = ParameterizedQuery::new(CREATE_TIMER_QUERY);
        query_builder.bind(QueryParameter::Str(timer.device));
        query_builder.bind(QueryParameter::DateTime(timer.end_time));
        query_builder.bind(QueryParameter::Str(timer.timer_type.as_string_name()));
        query_builder.bind(QueryParameter::Str(timer.updated_by));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await?;
        Ok(())
    }

    async fn delete_impl(&self, timer: ValidDeleteRequest) -> Result<(), DataStoreError> {
        info!(
            "Deleting alarm timer for device: {}, type: {}",
            timer.device,
            timer.timer_type.as_string_name()
        );

        let mut query_builder = ParameterizedQuery::new(DELETE_TIMER_QUERY);
        query_builder.bind(QueryParameter::Str(timer.device));
        query_builder.bind(QueryParameter::Str(timer.timer_type.as_string_name()));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await?;
        Ok(())
    }

    async fn read_bypass_reminders(
        &self,
        request: ValidReadRequest,
    ) -> Result<Vec<AlarmTimer>, DataStoreError> {
        info!(
            "Fetching bypass reminder timers for user: {} from the data store.",
            request.user
        );

        let mut query_builder = ParameterizedQuery::new(READ_USER_BYPASS_REMINDERS_QUERY);
        query_builder.bind(QueryParameter::Str(request.timer_type.as_string_name()));
        query_builder.bind(QueryParameter::Str(request.user));
        let rows = self
            .data_store
            .execute_parameterized_query(query_builder)
            .await?;
        rows_to_timers(rows)
    }

    async fn read_snooze_timers(&self) -> Result<Vec<AlarmTimer>, DataStoreError> {
        info!("Fetching all snooze timers from the data store.");
        let rows = self.data_store.execute_query(READ_SNOOZE_TIMERS).await?;
        rows_to_timers(rows)
    }

    async fn update_impl(&self, timer: ValidTimerInput) -> Result<(), DataStoreError> {
        info!(
            "Updating alarm timer for device: {}, type: {}",
            timer.device,
            timer.timer_type.as_string_name()
        );
        let mut query_builder = ParameterizedQuery::new(UPDATE_TIMER_QUERY);
        query_builder.bind(QueryParameter::DateTime(timer.end_time));
        query_builder.bind(QueryParameter::Str(timer.updated_by));
        query_builder.bind(QueryParameter::Str(timer.device));
        query_builder.bind(QueryParameter::Str(timer.timer_type.as_string_name()));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> AlarmTimerService
    for AlarmTimersServiceImpl<T, U, V>
{
    async fn create(&self, request: Request<AlarmTimer>) -> Result<Response<Empty>, Status> {
        let timer_input = request.into_inner();
        let timer = validate_timer_input(timer_input)?;

        self.create_impl(timer)
            .await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| {
                error!("Error setting alarm timer: {e:?}");
                Status::internal("Failed to set alarm timer. See server logs for details.")
            })
    }

    async fn delete(&self, request: Request<DeleteRequest>) -> Result<Response<Empty>, Status> {
        let delete_input = request.into_inner();
        let validated_request = validate_delete_request(delete_input)?;

        self.delete_impl(validated_request)
            .await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| {
                error!("Error deleting alarm timer: {e:?}");
                Status::internal("Failed to delete alarm timer. See server logs for details.")
            })
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<AlarmTimers>, Status> {
        let read_input = request.into_inner();
        let validated_request = validate_read_request(read_input)?;

        match validated_request.timer_type {
            KnownTimerType::Snooze => self.read_snooze_timers().await,
            KnownTimerType::Bypass => self.read_bypass_reminders(validated_request).await,
        }
        .map(|alarm_timers| Response::new(AlarmTimers { alarm_timers }))
        .map_err(|e| {
            error!("{e:?}");
            Status::internal("Failed to fetch alarm timers. See server logs for details.")
        })
    }

    async fn update(&self, request: Request<AlarmTimer>) -> Result<Response<Empty>, Status> {
        let timer_input = request.into_inner();
        let timer = validate_timer_input(timer_input)?;

        self.update_impl(timer)
            .await
            .map(|_| Response::new(Empty {}))
            .map_err(|e| {
                error!("Error updating alarm timer: {e:?}");
                Status::internal("Failed to update alarm timer. See server logs for details.")
            })
    }
}

struct ValidDeleteRequest {
    device: String,
    timer_type: KnownTimerType,
}

enum KnownTimerType {
    Bypass,
    Snooze,
}
impl KnownTimerType {
    fn as_string_name(&self) -> String {
        match self {
            KnownTimerType::Bypass => TimerType::BypassReminder.as_str_name(),
            KnownTimerType::Snooze => TimerType::Snooze.as_str_name(),
        }
        .to_owned()
    }
}
impl TryFrom<TimerType> for KnownTimerType {
    type Error = Status;

    fn try_from(value: TimerType) -> Result<Self, Self::Error> {
        match value {
            TimerType::BypassReminder => Ok(KnownTimerType::Bypass),
            TimerType::Snooze => Ok(KnownTimerType::Snooze),
            TimerType::Unknown => Err(Status::invalid_argument("Invalid \"timer_type\" value.")),
        }
    }
}

struct ValidReadRequest {
    timer_type: KnownTimerType,
    user: String,
}

struct ValidTimerInput {
    device: String,
    end_time: DateTime<Utc>,
    timer_type: KnownTimerType,
    updated_by: String,
}

fn rows_to_timers<T: DataVal, U: DataRow<T>>(
    rows: Vec<U>,
) -> Result<Vec<AlarmTimer>, DataStoreError> {
    rows.iter()
        .map(|row| {
            let device = row.get("device").to_string()?;
            let end_time = utils::datetime_to_timestamp(row.get("end_time").to_datetime()?);

            let timer_type_raw = row.get("timer_type").to_string()?;
            let timer_type =
                TimerType::from_str_name(&timer_type_raw).unwrap_or(TimerType::Unknown);

            let updated_at = utils::datetime_to_timestamp(row.get("updated_at").to_datetime()?);
            let updated_by = row.get("updated_by").to_string()?;
            Ok(AlarmTimer {
                device,
                end_time,
                timer_type: timer_type as i32,
                updated_at,
                updated_by,
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

fn timestamp_to_datetime(timestamp: Timestamp) -> Result<DateTime<Utc>, Status> {
    DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32).ok_or_else(|| {
        error!("Could not convert {timestamp:?} to Chrono DateTime - out of range");
        Status::invalid_argument(
            "Timestamp is not within the allowable range. Exceeds system limits.",
        )
    })
}

fn validate_delete_request(request: DeleteRequest) -> Result<ValidDeleteRequest, Status> {
    if request.device.is_empty() {
        return Err(Status::invalid_argument("\"device\" field is required."));
    }
    let timer_type = TimerType::try_from(request.timer_type).map_err(|err| {
        error!(
            "Could not parse {:?} into TimerType: {err:?}",
            request.timer_type
        );
        Status::invalid_argument("Invalid \"timer_type\" value.")
    })?;
    Ok(ValidDeleteRequest {
        device: request.device.to_lowercase(),
        timer_type: timer_type.try_into()?,
    })
}

fn validate_read_request(request: ReadRequest) -> Result<ValidReadRequest, Status> {
    let timer_type = TimerType::try_from(request.timer_type).map_err(|err| {
        error!(
            "Could not parse '{:?}' into TimerType: {err:?}",
            request.timer_type
        );
        Status::invalid_argument("Invalid \"timer_type\" value.")
    })?;
    let user = match timer_type {
        TimerType::BypassReminder => {
            if request.user.is_empty() {
                return Err(Status::invalid_argument(
                    "\"user\" field is required for reminder queries.",
                ));
            }
            request.user.to_lowercase()
        }
        _ => String::new(),
    };
    Ok(ValidReadRequest {
        timer_type: timer_type.try_into()?,
        user,
    })
}

fn validate_timer_input(timer: AlarmTimer) -> Result<ValidTimerInput, Status> {
    let device = if timer.device.is_empty() {
        return Err(Status::invalid_argument("\"device\" field is required."));
    } else {
        timer.device.to_lowercase()
    };

    let updated_by = if timer.updated_by.is_empty() {
        return Err(Status::invalid_argument(
            "\"updated_by\" field is required.",
        ));
    } else {
        timer.updated_by.to_lowercase()
    };

    let timestamp = match timer.end_time {
        None => return Err(Status::invalid_argument("\"end_time\" field is required.")),
        Some(time) => time,
    };

    Ok(ValidTimerInput {
        device,
        end_time: timestamp_to_datetime(timestamp)?,
        timer_type: TimerType::try_from(timer.timer_type)
            .map_err(|err| {
                error!("Could not parse timer type: {err:?}");
                Status::invalid_argument("Invalid \"timer_type\" value.")
            })?
            .try_into()?,
        updated_by,
    })
}
