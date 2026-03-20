//! User Layouts Module Queries
//!
//! The raw SQL for interacting with user layout data in the database.

/// SQL to grab all user layout data.
pub const GET_ALL_LAYOUTS_QUERY: &str = "
    SELECT 
        user_name,
        group_name
    FROM
        alarmsapp.user_layouts
    ORDER BY
        user_name,
        group_name
;";
