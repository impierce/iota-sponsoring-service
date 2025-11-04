use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use uuid::Uuid;

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
    client_handler: Arc<CqrsFramework<Client, CES>>,
    group_handler: Arc<CqrsFramework<Group, GES>>,
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
        client_handler: Arc<CqrsFramework<Client, CES>>,
        group_handler: Arc<CqrsFramework<Group, GES>>,
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

    pub async fn create_group(&self, group_id: Uuid, name: String) -> Result<GroupView> {
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
        };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        self.group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Group view not found after creating group `{group_id}`"))
    }

    pub async fn delete_group(&self, group_id: Uuid) -> Result<Uuid> {
        let client_list = self.client_list_view.load(CLIENT_LIST_VIEW_ID).await?;

        if let Some(client_list) = client_list {
            for (client_id, client_view) in client_list.into_inner().iter() {
                if let Some(client_group_id) = &client_view.group_id {
                    if client_group_id == &group_id {
                        let command = ClientCommand::RemoveClientFromGroup;

                        self.client_handler.execute(client_id, command).await?
                    }
                }
            }
        }

        let command = GroupCommand::DeleteGroup {
            group_id: group_id.clone(),
        };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        Ok(group_id)
    }

    pub async fn add_client_to_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupView> {
        let command = ClientCommand::AssignClientToGroup {
            group_id: group_id.clone(),
        };

        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let command = GroupCommand::AddClientToGroup {
            client_id: client_id.clone(),
        };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        self.group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after adding client `{client_id}` to group `{group_id}`"
                )
            })
    }

    pub async fn remove_client_from_group(
        &self,
        group_id: Uuid,
        client_id: Uuid,
    ) -> Result<GroupView> {
        let command = GroupCommand::RemoveClientFromGroup {
            client_id: client_id.clone(),
        };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let command = ClientCommand::RemoveClientFromGroup;

        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        self.group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after adding client `{client_id}` to group `{group_id}`"
                )
            })
    }

    pub async fn register_client(
        &self,
        client_id: Uuid,
        name: String,
        wallet_address: String,
    ) -> Result<ClientView> {
        let command = ClientCommand::RegisterClient {
            client_id: client_id.clone(),
            name,
            wallet_address,
        };

        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        self.client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Client view not found after registering client `{client_id}`"))
    }

    pub async fn remove_client(&self, client_id: Uuid) -> Result<Uuid> {
        let client_view = self.client_view.load(&client_id.to_string()).await?;
        let client_view = if let Some(client_view) = client_view {
            client_view
        } else {
            return Ok(client_id);
        };

        if let Some(group_id) = &client_view.group_id {
            let command = GroupCommand::RemoveClientFromGroup {
                client_id: client_id.clone(),
            };

            self.group_handler
                .execute(&group_id.to_string(), command)
                .await?;
        }

        let command = ClientCommand::RemoveClient {
            client_id: client_id.clone(),
        };

        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        Ok(client_id)
    }
}
