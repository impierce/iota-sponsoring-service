use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};

use crate::views::{
    client::ClientView,
    client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
    group::GroupView,
    group_list::GroupListView,
};

pub struct BalanceManagementService<CES, GES>
where
    CES: EventStore<Client>,
    GES: EventStore<Group>,
{
    client_handler: CqrsFramework<Client, CES>,
    group_handler: CqrsFramework<Group, GES>,
    client_view: Arc<dyn ViewRepository<ClientView, Client>>,
    client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
    group_view: Arc<dyn ViewRepository<GroupView, Group>>,
    // group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
}

impl<CES, GES> BalanceManagementService<CES, GES>
where
    CES: EventStore<Client> + 'static,
    GES: EventStore<Group> + 'static,
{
    pub fn new(
        client_handler: CqrsFramework<Client, CES>,
        group_handler: CqrsFramework<Group, GES>,
        client_view: Arc<dyn ViewRepository<ClientView, Client>>,
        client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
        group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            client_handler,
            group_handler,
            client_view,
            client_list_view,
            group_view,
            // group_list_view,
        }
    }

    pub async fn create_group(&self, id: String, name: String) -> Result<GroupView> {
        let command = GroupCommand::CreateGroup {
            id: id.clone(),
            name,
        };

        self.group_handler.execute(&id, command).await?;

        self.group_view
            .load(&id)
            .await?
            .ok_or_else(|| anyhow!("Group view not found after creating group `{id}`"))
    }

    pub async fn delete_group(&self, id: String) -> Result<String> {
        let client_list = self.client_list_view.load(CLIENT_LIST_VIEW_ID).await?;

        if let Some(client_list) = client_list {
            for (client_id, client_view) in client_list.into_inner().iter() {
                if let Some(client_group_id) = &client_view.group_id {
                    if client_group_id == &id {
                        let command = ClientCommand::RemoveClientFromGroup;

                        self.client_handler.execute(client_id, command).await?
                    }
                }
            }
        }

        let command = GroupCommand::DeleteGroup { id: id.clone() };

        self.group_handler.execute(&id, command).await?;

        Ok(id)
    }

    pub async fn add_client_to_group(&self, id: String, client_id: String) -> Result<GroupView> {
        let command = GroupCommand::AddClientToGroup {
            client_id: client_id.clone(),
        };

        self.group_handler.execute(&id, command).await?;

        let command = ClientCommand::AssignClientToGroup {
            group_id: id.clone(),
        };

        self.client_handler.execute(&client_id, command).await?;

        self.group_view.load(&id).await?.ok_or_else(|| {
            anyhow!("Group view not found after adding client `{client_id}` to group `{id}`")
        })
    }

    pub async fn remove_client_from_group(
        &self,
        id: String,
        client_id: String,
    ) -> Result<GroupView> {
        let command = GroupCommand::RemoveClientFromGroup {
            client_id: client_id.clone(),
        };

        self.group_handler.execute(&id, command).await?;

        let command = ClientCommand::RemoveClientFromGroup;

        self.client_handler.execute(&client_id, command).await?;

        self.group_view.load(&id).await?.ok_or_else(|| {
            anyhow!("Group view not found after adding client `{client_id}` to group `{id}`")
        })
    }

    pub async fn register_client(
        &self,
        id: String,
        name: String,
        wallet_address: String,
    ) -> Result<ClientView> {
        let command = ClientCommand::RegisterClient {
            id: id.clone(),
            name,
            wallet_address,
        };

        self.client_handler.execute(&id, command).await?;

        self.client_view
            .load(&id)
            .await?
            .ok_or_else(|| anyhow!("Client view not found after registering client `{id}`"))
    }

    pub async fn remove_client(&self, id: String) -> Result<String> {
        let client_view = self.client_view.load(&id).await?;
        let client_view = if let Some(client_view) = client_view {
            client_view
        } else {
            return Ok(id);
        };

        if let Some(group_id) = &client_view.group_id {
            let command = GroupCommand::RemoveClientFromGroup {
                client_id: id.clone(),
            };

            self.group_handler.execute(&group_id, command).await?;
        }

        let command = ClientCommand::RemoveClient { id: id.clone() };

        self.client_handler.execute(&id, command).await?;

        Ok(id)
    }
}
