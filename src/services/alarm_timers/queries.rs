//! Alarm Timers Module Queries
//!
//! Encapsulates the raw SQL used to query for alarm timers.

/// SQL to insert a new timer record into the database.
pub const CREATE_TIMER_QUERY: &str = "
    INSERT INTO alarmsapp.timers (device, end_time, timer_type, updated_by)
    VALUES (
        $1,
        $2, 
        (
            SELECT type_id 
            FROM alarmsapp.timer_types 
            WHERE type_name = $3
        ), 
        $4
    )
;";

/// SQL to delete a timer record from the database.
pub const DELETE_TIMER_QUERY: &str = "
    DELETE FROM alarmsapp.timers t
    INNER JOIN alarmsapp.timer_types tt ON t.timer_type = tt.type_id
    WHERE t.device = $1 AND tt.type_name = $2
;";

/// SQL to read bypass reminders for a specific user.
pub const READ_USER_BYPASS_REMINDERS_QUERY: &str = "
    SELECT
        device,
        end_time,
        type_name as timer_type,
        updated_at,
        updated_by
    FROM 
        alarmsapp.timers
    INNER JOIN alarmsapp.timer_types ON timer_type = type_id
    WHERE
        type_name = $1
        AND updated_by = $2
    ORDER BY
        device
;";

/// SQL to read the snooze timers.
pub const READ_SNOOZE_TIMERS: &str = "
    SELECT
        device,
        end_time,
        type_name as timer_type,
        updated_at,
        updated_by
    FROM 
        alarmsapp.timers
    INNER JOIN alarmsapp.timer_types ON timer_type = type_id
    WHERE
        type_name = 'TimerType_SNOOZE'
    ORDER BY
        device
;";

/// SQL to update an alarm timer record.
pub const UPDATE_TIMER_QUERY: &str = "
    UPDATE alarmsapp.timers
    SET end_time = $1, updated_by = $2
    INNER JOIN alarmsapp.timer_types ON timer_type = type_id
    WHERE device = $3 AND type_name = $4
;";
