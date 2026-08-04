//! User Layouts Module
//!
//! Contains logic for retrieval and storage of user alarm layout settings.

use crate::proto::{
    google::protobuf::Empty,
    services::alarm_user_layouts::{
        UserLayout, UserLayouts, user_layouts_service_server::UserLayoutsService,
    },
};
use queries::GET_ALL_LAYOUTS_QUERY;
use rust_db_lib::{DataRow, DataStore, DataStoreError, DataVal};
use std::{collections::HashMap, marker::PhantomData};
use tonic::{Request, Response, Status};
use tracing::{error, info};

mod queries;

#[cfg(test)]
mod tests;

/// A service wrapping a [`DataStore`] to provide alarm list layout information, and implementing the Protobuf-defined gRPC service.
pub struct UserLayoutsServiceImpl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> {
    data_store: V,
    _row_type: PhantomData<U>,
    _val_type: PhantomData<T>,
}

impl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> UserLayoutsServiceImpl<T, U, V> {
    pub fn new(data_store: V) -> Self {
        Self {
            data_store,
            _row_type: PhantomData,
            _val_type: PhantomData,
        }
    }

    /// Retrieves all top-level groups for each user. Used when generating the alarm screen display.
    async fn get_layouts(&self) -> Result<Vec<UserLayout>, DataStoreError> {
        info!("Query for user layouts");

        let rows = self.data_store.execute_query(GET_ALL_LAYOUTS_QUERY).await?;
        let mut layout_builder = HashMap::new();
        for row in rows {
            let user_name = row.get("user_name").to_string()?;
            let group_name = row.get("group_name").to_string()?;
            layout_builder
                .entry(user_name.clone())
                .or_insert_with(|| UserLayout {
                    user_name,
                    groups: Vec::new(),
                })
                .groups
                .push(group_name);
        }
        let mut sortable_rows = layout_builder.into_values().collect::<Vec<_>>();
        sortable_rows.sort_by(|a, b| a.user_name.cmp(&b.user_name));
        Ok(sortable_rows)
    }
}

#[tonic::async_trait]
impl<T: DataVal, U: DataRow<T>, V: DataStore<T, U>> UserLayoutsService
    for UserLayoutsServiceImpl<T, U, V>
{
    /// Translates query results from the DataStore into gRPC `UserLayouts` messages.
    async fn get_user_layouts(&self, _: Request<Empty>) -> Result<Response<UserLayouts>, Status> {
        self.get_layouts()
            .await
            .map(|layouts| Response::new(UserLayouts { layouts }))
            .map_err(|e| {
                error!("{e}");
                Status::internal("Failed to retrieve user layouts. See server logs for details.")
            })
    }
}
