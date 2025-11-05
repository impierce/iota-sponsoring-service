use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use tracing::{debug, info, instrument, warn};
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

    #[instrument(skip(self), fields(group_id = %group_id, name = %name))]
    pub async fn create_group(&self, group_id: Uuid, name: String) -> Result<GroupView> {
        info!("Creating new group");
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
        };

        debug!("Dispatching `CreateGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Group view not found after creating group `{group_id}`"))?;

        info!("Successfully created group");
        Ok(view)
    }

    #[instrument(skip(self), fields(group_id = %group_id))]
    pub async fn delete_group(&self, group_id: Uuid) -> Result<Uuid> {
        info!("Deleting group");
        let client_list = self.client_list_view.load(CLIENT_LIST_VIEW_ID).await?;

        if let Some(client_list) = client_list {
            debug!("Checking for clients that need to be removed from the group");
            for (client_id, client_view) in client_list.into_inner().iter() {
                if let Some(client_group_id) = &client_view.group_id {
                    if client_group_id == &group_id {
                        debug!(client_id=%client_id, "Removing client from group before deleting group");
                        let command = ClientCommand::RemoveClientFromGroup;

                        self.client_handler.execute(client_id, command).await?
                    }
                }
            }
        }

        let command = GroupCommand::DeleteGroup {
            group_id: group_id.clone(),
        };

        debug!("Dispatching `DeleteGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        info!("Successfully deleted group");
        Ok(group_id)
    }

    #[instrument(skip(self), fields(group_id = %group_id, client_id = %client_id))]
    pub async fn add_client_to_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupView> {
        info!("Adding client to group");
        let command = ClientCommand::AssignClientToGroup {
            group_id: group_id.clone(),
        };

        debug!("Dispatching `AssignClientToGroup` command to client aggregate");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let command = GroupCommand::AddClientToGroup {
            client_id: client_id.clone(),
        };

        debug!("Dispatching `AddClientToGroup` command to group aggregate");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after adding client `{client_id}` to group `{group_id}`"
                )
            })?;

        info!("Successfully added client to group");
        Ok(view)
    }

    #[instrument(skip(self), fields(group_id = %group_id, client_id = %client_id))]
    pub async fn remove_client_from_group(
        &self,
        group_id: Uuid,
        client_id: Uuid,
    ) -> Result<GroupView> {
        info!("Removing client from group");
        let command = GroupCommand::RemoveClientFromGroup {
            client_id: client_id.clone(),
        };

        debug!("Dispatching `RemoveClientFromGroup` command to group aggregate");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let command = ClientCommand::RemoveClientFromGroup;

        debug!("Dispatching `RemoveClientFromGroup` command to client aggregate");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after removing client `{client_id}` from group `{group_id}`"
                )
            })?;

        info!("Successfully removed client from group");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id, name = %name, wallet_address = %wallet_address))]
    pub async fn register_client(
        &self,
        client_id: Uuid,
        name: String,
        wallet_address: String,
    ) -> Result<ClientView> {
        info!("Registering new client");
        let command = ClientCommand::RegisterClient {
            client_id: client_id.clone(),
            name,
            wallet_address,
        };

        debug!("Dispatching `RegisterClient` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!("Client view not found after registering client `{client_id}`")
            })?;

        info!("Successfully registered client");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn remove_client(&self, client_id: Uuid) -> Result<Uuid> {
        info!("Removing client");
        let client_view = self.client_view.load(&client_id.to_string()).await?;
        let client_view = if let Some(client_view) = client_view {
            client_view
        } else {
            warn!("Attempted to remove a client that does not exist. No action taken.");
            return Ok(client_id);
        };

        if let Some(group_id) = &client_view.group_id {
            debug!(group_id = %group_id, "Client belongs to a group, removing from group first");
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

        debug!("Dispatching `RemoveClient` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        info!("Successfully removed client");
        Ok(client_id)
    }
}
