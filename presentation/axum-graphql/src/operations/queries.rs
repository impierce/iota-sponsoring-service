use anyhow::Result;
use application::views::{
    client::ClientView,
    client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
    group::GroupView,
    group_list::{GROUP_LIST_VIEW_ID, GroupListView},
};
use async_graphql::Object;
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::ViewRepository;
use mongo_es::MongoViewRepository;
use std::sync::Arc;
use uuid::Uuid;

use crate::operations::{ClientDto, GroupDto, get_shared_balance};

#[derive(Clone)]
pub struct QueryRoot {
    client_view: Arc<MongoViewRepository<ClientView, Client>>,
    client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
    group_view: Arc<MongoViewRepository<GroupView, Group>>,
    group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,
}

impl QueryRoot {
    pub fn new(
        client_view: Arc<MongoViewRepository<ClientView, Client>>,
        client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
        group_view: Arc<MongoViewRepository<GroupView, Group>>,
        group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            client_view,
            client_list_view,
            group_view,
            group_list_view,
        }
    }
}

#[Object]
impl QueryRoot {
    /// Returns the token balance (decreases over time, resets to 50000 when it hits 0)
    async fn token_balance(&self) -> i32 {
        let balance = get_shared_balance().read().await;
        balance.value
    }

    /// Returns a client by ID
    async fn get_client(&self, client_id: Uuid) -> Result<Option<ClientDto>> {
        Ok(self
            .client_view
            .load(&client_id.to_string())
            .await?
            .and_then(|client_view| {
                (!client_view.is_deleted).then(|| ClientDto::from(client_view))
            }))
    }

    /// Returns the list of all clients
    async fn get_client_list(&self) -> Result<Vec<ClientDto>> {
        Ok(self
            .client_list_view
            .load(CLIENT_LIST_VIEW_ID)
            .await?
            .map(|client_list_view| {
                client_list_view
                    .into_inner()
                    .values()
                    .cloned()
                    .filter_map(|client_view| {
                        (!client_view.is_deleted).then(|| ClientDto::from(client_view))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Returns a group by ID
    async fn get_group(&self, group_id: Uuid) -> Result<Option<GroupDto>> {
        Ok(self
            .group_view
            .load(&group_id.to_string())
            .await?
            .and_then(|group_view| (!group_view.is_deleted).then(|| GroupDto::from(group_view))))
    }

    /// Returns the list of all groups
    async fn get_group_list(&self) -> Result<Vec<GroupDto>> {
        Ok(self
            .group_list_view
            .load(GROUP_LIST_VIEW_ID)
            .await?
            .map(|group_list_view| {
                group_list_view
                    .into_inner()
                    .values()
                    .cloned()
                    .filter_map(|group_view| {
                        (!group_view.is_deleted).then(|| GroupDto::from(group_view))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
}
