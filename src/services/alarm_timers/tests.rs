//! Alarm Timers Module Tests

use super::*;
use rust_db_lib::testing_utils::{TestDataStore, TestVal};

#[derive(Clone)]
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
            "end_time" => val.test_datetime = Some(self.end_time),
            "timer_type" => val.test_string = Some(self.timer_type.clone()),
            "updated_at" => val.test_datetime = Some(self.updated_at),
            "updated_by" => val.test_string = Some(self.updated_by.clone()),
            _ => (),
        };
        val
    }
}

#[tokio::test]
async fn test_read_bypass_reminders() {
    let test_time = Utc::now();
    let test_row = TestRow {
        device: "Device1".to_string(),
        end_time: test_time,
        timer_type: TimerType::BypassReminder.as_str_name().to_string(),
        updated_at: test_time,
        updated_by: "UserA".to_string(),
    };
    let data_store = TestDataStore {
        data: vec![test_row.clone()],
    };
    let service = AlarmTimersServiceImpl::new(data_store);
    let result = service
        .read_bypass_reminders(ValidReadRequest {
            timer_type: KnownTimerType::Bypass,
            user: "UserA".to_string(),
        })
        .await;
    assert!(result.is_ok());

    let timers = result.expect("read_bypass_reminders should succeed");
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
        end_time: test_time,
        timer_type: TimerType::Snooze.as_str_name().to_string(),
        updated_at: test_time,
        updated_by: "UserA".to_string(),
    };
    let data_store = TestDataStore {
        data: vec![test_row.clone()],
    };

    let service = AlarmTimersServiceImpl::new(data_store);
    let result = service
        .read(Request::new(ReadRequest {
            timer_type: TimerType::Snooze as i32,
            user: String::new(),
        }))
        .await;

    assert!(result.is_ok());

    let timers = result
        .expect("read snooze timers should succeed")
        .into_inner()
        .alarm_timers;
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
    invalid_timer.end_time = valid_time;
    let result = validate_timer_input(invalid_timer);
    assert!(result.is_err());

    let mut invalid_timer = invalid_alarm_timer();
    invalid_timer.device = valid_device.clone();
    invalid_timer.end_time = valid_time;
    invalid_timer.timer_type = 897;
    let result = validate_timer_input(invalid_timer);
    assert!(result.is_err());

    let mut invalid_timer = invalid_alarm_timer();
    invalid_timer.device = valid_device.clone();
    invalid_timer.end_time = valid_time;
    invalid_timer.timer_type = TimerType::Snooze as i32;
    let result = validate_timer_input(invalid_timer);
    assert!(result.is_err());
}

#[test]
fn test_timestamp_to_datetime_out_of_range() {
    let invalid_timestamp = Timestamp {
        seconds: i64::MAX,
        nanos: i32::MAX,
    };
    let result = timestamp_to_datetime(invalid_timestamp);
    assert!(result.is_err());
}
