//! Alarm Groups Module Queries
//!
//! Contains the raw SQL queries to be formatted by the Alarm Group Module logic

/// A macro to generate the format string when querying the details of specific groups.
macro_rules! GROUP_DETAILS_QUERY {
    () => {
        "
            WITH RECURSIVE members AS (
                SELECT 
                    group_name,
                    member_name,
                    member_is_group
                FROM 
                    alarmsapp.group_membership
                WHERE
                    group_name IN ({})
                UNION ALL
                SELECT
                    gm.group_name,
                    gm.member_name,
                    gm.member_is_group,
                FROM 
                    alarmsapp.group_membership gm 
                    INNER JOIN members m 
                        ON gm.group_name = m.member_name
            )
            SELECT 
                g.group_name,
                g.description,
                g.updated_at,
                g.updated_by,
                EXISTS (
                    SELECT 
                    FROM alarmsapp.user_layouts u
                    WHERE g.group_name = u.group_name
                ) AS group_is_user_category,
                m.member_name,
                m.member_is_group
            FROM
                alarmsapp.groups g
                INNER JOIN members m
                    ON g.group_name = m.group_name
            ORDER BY 
                g.group_name, 
                m.member_name
            ;
        "
    };
}
pub(crate) use GROUP_DETAILS_QUERY;

/// A query to retrieve all metadata for alarm groups.
pub const ALL_GROUPS_METADATA_QUERY: &str = "
    SELECT 
        g.group_name,
        g.description,
        g.updated_at,
        g.updated_by,
        EXISTS (
            SELECT
            FROM alarmsapp.user_layouts u
            WHERE g.group_name = u.group_name
        ) AS group_is_user_category
    FROM 
        alarmsapp.groups g
    ORDER BY
        updated_by,
        group_name
    ;
";
