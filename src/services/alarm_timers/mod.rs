use chrono::{DateTime, Utc};

mod proto {
    tonic::include_proto!("services.alarm_timers");
}
pub use proto::alarm_timer_service_server::AlarmTimerServiceServer;
use proto::{
    AlarmTimer, AlarmTimers, DeleteRequest, ReadRequest, TimerType,
    alarm_timer_service_server::AlarmTimerService,
};

use rust_db_lib::{
    DataRow, DataStore, DataStoreError, DataVal, ParameterizedQuery, QueryParameter,
};

use std::marker::PhantomData;

use tonic::{Request, Response, Status};
use tracing::{error, info};

use crate::utils;

struct ValidDeleteRequest {
    device: String,
    timer_type: String,
}

struct ValidReadRequest {
    timer_type: TimerType,
    user: String,
}

struct ValidTimerInput {
    device: String,
    end_time: DateTime<Utc>,
    timer_type: TimerType,
    updated_by: String,
}

fn rows_to_timers<T: DataVal, U: DataRow<T>>(
    rows: Vec<U>,
) -> Result<Vec<AlarmTimer>, DataStoreError> {
    let mut timers = Vec::new();
    for row in rows {
        let device = row.get("device").to_string()?;
        let end_time = utils::datetime_to_timestamp(row.get("end_time").to_datetime()?);

        let timer_type_raw = row.get("timer_type").to_string()?;
        let timer_type = TimerType::from_str_name(&timer_type_raw).unwrap_or(TimerType::Unknown);

        let updated_at = utils::datetime_to_timestamp(row.get("updated_at").to_datetime()?);
        let updated_by = row.get("updated_by").to_string()?;
        timers.push(AlarmTimer {
            device,
            end_time,
            timer_type: timer_type as i32,
            updated_at,
            updated_by,
        });
    }
    Ok(timers)
}

fn timestamp_to_datetime(timestamp: &prost_types::Timestamp) -> Result<DateTime<Utc>, Status> {
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
    if request.timer_type == TimerType::Unknown as i32 {
        return Err(Status::invalid_argument("Invalid \"timer_type\" value."));
    }
    let timer_type = TimerType::try_from(request.timer_type).map_err(|err| {
        error!(
            "Could not parse {:?} into TimerType: {err:?}",
            request.timer_type
        );
        Status::invalid_argument("Invalid \"timer_type\" value.")
    })?;
    Ok(ValidDeleteRequest {
        device: request.device,
        timer_type: timer_type.as_str_name().to_string(),
    })
}

fn validate_read_request(request: ReadRequest) -> Result<ValidReadRequest, Status> {
    if request.timer_type == TimerType::Unknown as i32 {
        return Err(Status::invalid_argument("Invalid \"timer_type\" value."));
    }
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
            request.user
        }
        _ => String::default(),
    };
    Ok(ValidReadRequest { timer_type, user })
}

fn validate_timer_input(timer: AlarmTimer) -> Result<ValidTimerInput, Status> {
    if timer.device.is_empty() {
        return Err(Status::invalid_argument("\"device\" field is required."));
    }
    if timer.timer_type == TimerType::Unknown as i32 {
        return Err(Status::invalid_argument("Invalid \"timer_type\" value."));
    }
    if timer.updated_by.is_empty() {
        return Err(Status::invalid_argument(
            "\"updated_by\" field is required.",
        ));
    }
    if timer.end_time.is_none() {
        return Err(Status::invalid_argument("\"end_time\" field is required."));
    }
    Ok(ValidTimerInput {
        device: timer.device,
        end_time: timestamp_to_datetime(&timer.end_time.unwrap())?,
        timer_type: TimerType::try_from(timer.timer_type).map_err(|err| {
            error!("Could not parse timer type: {err:?}");
            Status::invalid_argument("Invalid \"timer_type\" value.")
        })?,
        updated_by: timer.updated_by,
    })
}

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
            timer.timer_type.as_str_name()
        );
        let query = "
            INSERT INTO alarmsapp.timers (device, end_time, timer_type, updated_by)
            VALUES ($1, $2, $3, $4)
            ;
            "
        .to_string();
        let mut query_builder = ParameterizedQuery::new(query);
        query_builder.bind(QueryParameter::STR(timer.device));
        query_builder.bind(QueryParameter::DATETIME(timer.end_time));
        query_builder.bind(QueryParameter::STR(
            timer.timer_type.as_str_name().to_string(),
        ));
        query_builder.bind(QueryParameter::STR(timer.updated_by));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await
            .map(|_| ())
    }

    async fn delete_impl(&self, timer: ValidDeleteRequest) -> Result<(), DataStoreError> {
        info!(
            "Deleting alarm timer for device: {}, type: {}",
            timer.device, timer.timer_type
        );

        let query = "
            DELETE FROM alarmsapp.timers
            WHERE device = $1 AND timer_type = $2
            ;
            "
        .to_string();
        let mut query_builder = ParameterizedQuery::new(query);
        query_builder.bind(QueryParameter::STR(timer.device));
        query_builder.bind(QueryParameter::STR(timer.timer_type));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await
            .map(|_| ())
    }

    async fn read_bypass_reminders(
        &self,
        request: ValidReadRequest,
    ) -> Result<Vec<AlarmTimer>, DataStoreError> {
        info!(
            "Fetching bypass reminder timers for user: {} from the data store.",
            request.user
        );

        let query = "
            SELECT
                device,
                end_time,
                timer_type,
                updated_at,
                updated_by
            FROM 
                alarmsapp.timers
            WHERE
                timer_type = $1
                AND updated_by = $2
            ORDER BY
                device
            ;
            ";
        let mut query_builder = ParameterizedQuery::new(query.to_string());
        query_builder.bind(QueryParameter::STR(
            TimerType::BypassReminder.as_str_name().to_string(),
        ));
        query_builder.bind(QueryParameter::STR(request.user));
        let rows = self
            .data_store
            .execute_parameterized_query(query_builder)
            .await?;
        rows_to_timers(rows)
    }

    async fn read_snooze_timers(&self) -> Result<Vec<AlarmTimer>, DataStoreError> {
        info!("Fetching all snooze timers from the data store.");

        let query = "
            SELECT
                device,
                end_time,
                timer_type,
                updated_at,
                updated_by
            FROM 
                alarmsapp.timers
            WHERE
                timer_type = 'TimerType_SNOOZE'
            ORDER BY
                device
            ;
            ";
        let rows = self.data_store.execute_query(query).await?;
        rows_to_timers(rows)
    }

    async fn update_impl(&self, timer: ValidTimerInput) -> Result<(), DataStoreError> {
        info!(
            "Updating alarm timer for device: {}, type: {}",
            timer.device,
            timer.timer_type.as_str_name()
        );
        let query = "
            UPDATE alarmsapp.timers
            SET end_time = $1, updated_by = $2
            WHERE device = $3 AND timer_type = $4
            ;
            "
        .to_string();
        let mut query_builder = ParameterizedQuery::new(query);
        query_builder.bind(QueryParameter::DATETIME(timer.end_time));
        query_builder.bind(QueryParameter::STR(timer.updated_by));
        query_builder.bind(QueryParameter::STR(timer.device));
        query_builder.bind(QueryParameter::STR(
            timer.timer_type.as_str_name().to_string(),
        ));
        self.data_store
            .execute_parameterized_query(query_builder)
            .await
            .map(|_| ())
    }
}

#[tonic::async_trait]
impl<T: DataVal + 'static, U: DataRow<T> + 'static, V: DataStore<T, U> + 'static> AlarmTimerService
    for AlarmTimersServiceImpl<T, U, V>
{
    async fn create(&self, request: Request<AlarmTimer>) -> Result<Response<()>, Status> {
        let timer_input = request.into_inner();
        let timer = validate_timer_input(timer_input)?;

        match self.create_impl(timer).await {
            Ok(_) => Ok(Response::new(())),
            Err(e) => {
                error!("Error setting alarm timer: {e:?}");
                Err(Status::internal(
                    "Failed to set alarm timer. See server logs for details.",
                ))
            }
        }
    }

    async fn delete(&self, request: Request<DeleteRequest>) -> Result<Response<()>, Status> {
        let delete_input = request.into_inner();
        let validated_request = validate_delete_request(delete_input)?;
        match self.delete_impl(validated_request).await {
            Ok(_) => Ok(Response::new(())),
            Err(e) => {
                error!("Error deleting alarm timer: {e:?}");
                Err(Status::internal(
                    "Failed to delete alarm timer. See server logs for details.",
                ))
            }
        }
    }

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<AlarmTimers>, Status> {
        let read_input = request.into_inner();
        let validated_request = validate_read_request(read_input)?;

        let result = if let TimerType::Snooze = validated_request.timer_type {
            self.read_snooze_timers().await
        } else {
            self.read_bypass_reminders(validated_request).await
        };

        match result {
            Ok(alarm_timers) => Ok(Response::new(AlarmTimers { alarm_timers })),
            Err(e) => {
                error!("{e:?}");
                Err(Status::internal(
                    "Failed to fetch alarm timers. See server logs for details.",
                ))
            }
        }
    }

    async fn update(&self, request: Request<AlarmTimer>) -> Result<Response<()>, Status> {
        let timer_input = request.into_inner();
        let timer = validate_timer_input(timer_input)?;

        match self.update_impl(timer).await {
            Ok(_) => Ok(Response::new(())),
            Err(e) => {
                error!("Error updating alarm timer: {e:?}");
                Err(Status::internal(
                    "Failed to update alarm timer. See server logs for details.",
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{DateTime, Utc};

    use rust_db_lib::test_utils::{TestDataStore, TestVal};

    struct TestRow {
        device: String,
        end_time: DateTime<Utc>,
        timer_type: String,
        updated_at: DateTime<Utc>,
        updated_by: String,
    }
    impl DataRow<TestVal> for TestRow {
        fn get(&self, col: &str) -> TestVal {
            let mut val = TestVal::new();
            match col {
                "device" => val.test_string = Some(self.device.clone()),
                "end_time" => val.test_datetime = Some(self.end_time.clone()),
                "timer_type" => val.test_string = Some(self.timer_type.clone()),
                "updated_at" => val.test_datetime = Some(self.updated_at.clone()),
                "updated_by" => val.test_string = Some(self.updated_by.clone()),
                _ => (),
            };
            val
        }
    }
    impl Clone for TestRow {
        fn clone(&self) -> Self {
            TestRow {
                device: self.device.clone(),
                end_time: self.end_time.clone(),
                timer_type: self.timer_type.clone(),
                updated_at: self.updated_at.clone(),
                updated_by: self.updated_by.clone(),
            }
        }
    }

    #[tokio::test]
    async fn test_read_bypass_reminders() {
        let test_time = Utc::now();
        let test_row = TestRow {
            device: "Device1".to_string(),
            end_time: test_time.clone(),
            timer_type: TimerType::BypassReminder.as_str_name().to_string(),
            updated_at: test_time.clone(),
            updated_by: "UserA".to_string(),
        };
        let data_store = TestDataStore {
            data: vec![test_row.clone()],
        };
        let service = AlarmTimersServiceImpl::new(data_store);
        let result = service
            .read_bypass_reminders(ValidReadRequest {
                timer_type: TimerType::BypassReminder,
                user: "UserA".to_string(),
            })
            .await;
        assert!(result.is_ok());

        let timers = result.unwrap();
        assert_eq!(timers.len(), 1);
        let timer = &timers[0];
        assert_eq!(timer.device, test_row.device);
        assert_eq!(
            timer.end_time,
            utils::datetime_to_timestamp(test_row.end_time)
        );
        assert_eq!(timer.timer_type, TimerType::BypassReminder as i32);
        assert_eq!(
            timer.updated_at,
            utils::datetime_to_timestamp(test_row.updated_at)
        );
        assert_eq!(timer.updated_by, test_row.updated_by);
    }

    #[tokio::test]
    async fn test_read_snooze_timers() {
        let test_time = Utc::now();
        let test_row = TestRow {
            device: "Device1".to_string(),
            end_time: test_time.clone(),
            timer_type: TimerType::Snooze.as_str_name().to_string(),
            updated_at: test_time.clone(),
            updated_by: "UserA".to_string(),
        };
        let data_store = TestDataStore {
            data: vec![test_row.clone()],
        };

        let service = AlarmTimersServiceImpl::new(data_store);
        let result = service
            .read(Request::new(ReadRequest {
                timer_type: TimerType::Snooze as i32,
                user: String::default(),
            }))
            .await;

        assert!(result.is_ok());

        let timers = result.unwrap().into_inner().alarm_timers;
        assert_eq!(timers.len(), 1);

        let timer = &timers[0];
        assert_eq!(timer.device, test_row.device);
        assert_eq!(
            timer.end_time,
            utils::datetime_to_timestamp(test_row.end_time)
        );
        assert_eq!(timer.timer_type, TimerType::Snooze as i32);
        assert_eq!(
            timer.updated_at,
            utils::datetime_to_timestamp(test_row.updated_at)
        );
        assert_eq!(timer.updated_by, test_row.updated_by);
    }

    #[tokio::test]
    async fn test_create_timer() {
        let data_store = TestDataStore::<TestRow> { data: vec![] };
        let service = AlarmTimersServiceImpl::new(data_store);

        let test_time = utils::datetime_to_timestamp(Utc::now());
        let alarm_timer = AlarmTimer {
            device: "Device2".to_string(),
            end_time: test_time,
            timer_type: TimerType::BypassReminder as i32,
            updated_at: None,
            updated_by: "UserB".to_string(),
        };

        let result = service.create(Request::new(alarm_timer)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_timer() {
        let data_store = TestDataStore::<TestRow> { data: vec![] };
        let service = AlarmTimersServiceImpl::new(data_store);

        let delete_request = DeleteRequest {
            device: "Device3".to_string(),
            timer_type: TimerType::Snooze as i32,
        };

        let result = service.delete(Request::new(delete_request)).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_timer() {
        let data_store = TestDataStore::<TestRow> { data: vec![] };
        let service = AlarmTimersServiceImpl::new(data_store);
        let test_time = utils::datetime_to_timestamp(Utc::now());
        let alarm_timer = AlarmTimer {
            device: "Device4".to_string(),
            end_time: test_time,
            timer_type: TimerType::BypassReminder as i32,
            updated_at: None,
            updated_by: "UserC".to_string(),
        };
        let result = service.update(Request::new(alarm_timer)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_delete_request() {
        let invalid_request = DeleteRequest {
            device: "".to_string(),
            timer_type: TimerType::BypassReminder as i32,
        };
        let result = validate_delete_request(invalid_request);
        assert!(result.is_err());

        let invalid_request = DeleteRequest {
            device: "Device1".to_string(),
            timer_type: TimerType::Unknown as i32,
        };
        let result = validate_delete_request(invalid_request);
        assert!(result.is_err());

        let invalid_request = DeleteRequest {
            device: "Device1".to_string(),
            timer_type: 12345,
        };
        let result = validate_delete_request(invalid_request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_read_request() {
        let invalid_request = ReadRequest {
            timer_type: TimerType::Unknown as i32,
            user: "UserA".to_string(),
        };
        let result = validate_read_request(invalid_request);
        assert!(result.is_err());

        let invalid_request = ReadRequest {
            timer_type: TimerType::BypassReminder as i32,
            user: "".to_string(),
        };
        let result = validate_read_request(invalid_request);
        assert!(result.is_err());

        let invalid_request = ReadRequest {
            timer_type: 67890,
            user: "UserA".to_string(),
        };
        let result = validate_read_request(invalid_request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_timer_input() {
        let valid_device = "Device3".to_string();
        let valid_time = utils::datetime_to_timestamp(Utc::now());

        fn invalid_alarm_timer() -> AlarmTimer {
            AlarmTimer {
                device: "".to_string(),
                end_time: None,
                timer_type: TimerType::Unknown as i32,
                updated_at: None,
                updated_by: "".to_string(),
            }
        }

        let result = validate_timer_input(invalid_alarm_timer());
        assert!(result.is_err());

        let mut invalid_timer = invalid_alarm_timer();
        invalid_timer.device = valid_device.clone();
        let result = validate_timer_input(invalid_timer);
        assert!(result.is_err());

        let mut invalid_timer = invalid_alarm_timer();
        invalid_timer.device = valid_device.clone();
        invalid_timer.end_time = valid_time.clone();
        let result = validate_timer_input(invalid_timer);
        assert!(result.is_err());

        let mut invalid_timer = invalid_alarm_timer();
        invalid_timer.device = valid_device.clone();
        invalid_timer.end_time = valid_time.clone();
        invalid_timer.timer_type = 897;
        let result = validate_timer_input(invalid_timer);
        assert!(result.is_err());

        let mut invalid_timer = invalid_alarm_timer();
        invalid_timer.device = valid_device.clone();
        invalid_timer.end_time = valid_time.clone();
        invalid_timer.timer_type = TimerType::Snooze as i32;
        let result = validate_timer_input(invalid_timer);
        assert!(result.is_err());
    }

    #[test]
    fn test_timestamp_to_datetime_out_of_range() {
        let invalid_timestamp = prost_types::Timestamp {
            seconds: i64::MAX,
            nanos: i32::MAX,
        };
        let result = timestamp_to_datetime(&invalid_timestamp);
        assert!(result.is_err());
    }
}
