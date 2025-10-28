use application::views::{
    client::ClientView, client_list::ClientListView, group::GroupView, group_list::GroupListView,
};
use async_graphql::Object;
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::ViewRepository;
use mongo_es::MongoViewRepository;
use std::sync::Arc;

use crate::operations::{ClientUpdate, GroupUpdate, get_shared_balance};

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
    async fn get_client(&self, client_id: String) -> Option<ClientUpdate> {
        match self.client_view.load(&client_id).await {
            Ok(Some(view)) => Some(view.into()),
            _ => None,
        }
    }

    /// Returns the list of all clients
    async fn get_client_list(&self) -> Option<Vec<ClientUpdate>> {
        match self.client_list_view.load("client_list").await {
            Ok(Some(view)) => {
                let clients: Vec<ClientUpdate> = view
                    .into_inner()
                    .values()
                    .cloned()
                    .map(ClientUpdate::from)
                    .collect();
                Some(clients)
            }
            _ => None,
        }
    }

    /// Returns a group by ID
    async fn get_group(&self, group_id: String) -> Option<GroupUpdate> {
        match self.group_view.load(&group_id).await {
            Ok(Some(view)) => Some(view.into()),
            _ => None,
        }
    }

    /// Returns the list of all groups
    async fn get_group_list(&self) -> Option<Vec<GroupUpdate>> {
        match self.group_list_view.load("group_list").await {
            Ok(Some(view)) => {
                let groups: Vec<GroupUpdate> = view
                    .into_inner()
                    .values()
                    .cloned()
                    .map(GroupUpdate::from)
                    .collect();
                Some(groups)
            }
            _ => None,
        }
    }
}
