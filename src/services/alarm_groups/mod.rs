//! Alarm Groups Module
//!
//! Contains logic for retrieving and updating alarm groups.
//!
pub use proto::alarm_group_service_server::AlarmGroupServiceServer;

use crate::utils;
use proto::{
    AlarmGroup, AlarmGroupMetadata, AlarmGroupMetadatum, AlarmGroups, GroupsRequest,
    alarm_group_service_server::AlarmGroupService,
};
use queries::{ALL_GROUPS_METADATA_QUERY, GROUP_DETAILS_QUERY};
use rust_db_lib::{
    DataRow, DataStore, DataStoreError, DataVal, ParameterizedQuery, QueryParameter,
};
use std::{cmp::Ordering, collections::HashMap, marker::PhantomData};
use tonic::{Request, Response, Status};
use tracing::{error, info};

mod proto {
    tonic::include_proto!("services.alarm_groups");
}
mod queries;

#[cfg(test)]
mod tests;

/// A service wrapping a [`DataStore`] to provide alarm group information, and implementing the Protobuf-defined gRPC service.
pub struct AlarmGroupsServiceImpl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> {
    data_store: V,
    _row_type: PhantomData<U>,
    _val_type: PhantomData<T>,
}
impl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> AlarmGroupsServiceImpl<T, U, V> {
    pub fn new(data_store: V) -> Self {
        Self {
            data_store,
            _row_type: PhantomData,
            _val_type: PhantomData,
        }
    }

    /// Retrieves all alarm group metadata.
    async fn get_all_metadata(&self) -> Result<Vec<AlarmGroupMetadatum>, DataStoreError> {
        info!("Query for all alarm group metadata ");
        let query_result = self
            .data_store
            .execute_query(ALL_GROUPS_METADATA_QUERY)
            .await?;
        let mut metadata = rows_to_metadata(query_result)?;
        metadata.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(metadata)
    }

    async fn get_requested_groups(
        &self,
        specified_groups: Vec<String>,
    ) -> Result<Vec<AlarmGroup>, DataStoreError> {
        info!(
            "Query for alarm groups {:?}, and associated devices ",
            specified_groups
        );

        let needed_placeholders: Vec<String> = (1..specified_groups.len())
            .map(|index| format!("${}", index))
            .collect();

        // Uses a recursive Common Table Expression (CTE) to build the result rows down to the device level.
        // That is, users can specify the top-level groups they want, and this SQL query will return all the
        // child, grandchild, etc. objects under it.
        let alarm_group_query = format!(GROUP_DETAILS_QUERY!(), needed_placeholders.join(", "));

        let mut query_builder = ParameterizedQuery::new(alarm_group_query);
        for group in specified_groups {
            query_builder.bind(QueryParameter::STR(group));
        }

        let query_result = self
            .data_store
            .execute_parameterized_query(query_builder)
            .await?;
        let mut groups = rows_to_groups(query_result)?;
        groups.sort_by(sort_groups);

        Ok(groups)
    }
}

#[tonic::async_trait]
impl<T: DataVal + 'static, U: DataRow<T> + 'static, V: DataStore<T, U> + 'static> AlarmGroupService
    for AlarmGroupsServiceImpl<T, U, V>
{
    /// Retrieves [`AlarmGroupMetadata`] for all alarm groups.
    async fn get_group_metadata(
        &self,
        _: Request<()>,
    ) -> Result<Response<AlarmGroupMetadata>, Status> {
        match self.get_all_metadata().await {
            Ok(metadata) => Ok(Response::new(AlarmGroupMetadata { metadata })),
            Err(e) => {
                error!("{}", e);
                Err(Status::internal(
                    "Failed to retrieve alarm group metadata. See server logs for details.",
                ))
            }
        }
    }

    /// Retrieves full [`AlarmGroup`] information for the specified groups.
    async fn get_groups(
        &self,
        request: Request<GroupsRequest>,
    ) -> Result<Response<AlarmGroups>, Status> {
        let requested_groups = request.into_inner().groups;
        match self.get_requested_groups(requested_groups).await {
            Ok(alarm_groups) => Ok(Response::new(AlarmGroups { alarm_groups })),
            Err(e) => {
                error!("{}", e);
                Err(Status::internal(
                    "Failed to retrieve alarm groups. See server logs for details.",
                ))
            }
        }
    }
}

fn create_metadatum<T: DataVal, U: DataRow<T>>(
    row: &U,
    name: String,
) -> Result<AlarmGroupMetadatum, DataStoreError> {
    let description = row.get("description").to_string()?;
    let updated_at = utils::datetime_to_timestamp(row.get("updated_at").to_datetime()?);
    let updated_by = row.get("updated_by").to_string()?;
    let is_user_category = row.get("group_is_user_category").to_bool()?;
    Ok(AlarmGroupMetadatum {
        name,
        description,
        updated_at,
        updated_by,
        is_user_category,
    })
}

fn rows_to_metadata<T: DataVal, U: DataRow<T>>(
    rows: Vec<U>,
) -> Result<Vec<AlarmGroupMetadatum>, DataStoreError> {
    let mut metadata = Vec::new();
    for row in &rows {
        let metadatum = create_metadatum(row, row.get("group_name").to_string()?)?;
        metadata.push(metadatum);
    }
    Ok(metadata)
}

fn rows_to_groups<T: DataVal, U: DataRow<T>>(
    rows: Vec<U>,
) -> Result<Vec<AlarmGroup>, DataStoreError> {
    let mut group_builder = HashMap::new();
    for row in &rows {
        let group_name = row.get("group_name").to_string()?;
        let alarm_group = group_builder
            .entry(group_name.clone())
            .or_insert_with(|| AlarmGroup {
                metadata: create_metadatum(row, group_name).ok(),
                devices: Vec::new(),
                groups: Vec::new(),
            });
        let member_name = row.get("member_name").to_string()?;
        if row.get("member_is_group").to_bool()? {
            alarm_group.groups.push(member_name);
        } else {
            alarm_group.devices.push(member_name);
        }
    }
    Ok(group_builder.into_values().collect())
}

fn sort_groups(a: &AlarmGroup, b: &AlarmGroup) -> Ordering {
    match a.metadata.as_ref() {
        Some(group) => match b.metadata.as_ref() {
            Some(other_group) => group.name.cmp(&other_group.name),
            None => Ordering::Less,
        },
        None => Ordering::Greater,
    }
}
