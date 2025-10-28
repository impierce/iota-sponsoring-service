use std::sync::Arc;

use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};

use crate::views::{
    client::ClientView, client_list::ClientListView, group::GroupView, group_list::GroupListView,
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
    // group_view: Arc<dyn ViewRepository<GroupView, Group>>,
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
        _group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            client_handler,
            group_handler,
            client_view,
            client_list_view,
            // group_view,
            // group_list_view,
        }
    }

    pub async fn create_group(&self, group_id: String, name: String) -> Result<bool, String> {
        let command = GroupCommand::CreateGroup {
            id: group_id.clone(),
            name,
        };

        match self.group_handler.execute(&group_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to create group: {:?}", e)),
        }
    }

    pub async fn delete_group(&self, group_id: String) -> Result<bool, String> {
        let client_list = self
            .client_list_view
            .load("client_list")
            .await
            .map_err(|e| {
                format!(
                    "Failed to load client list view while deleting group {}: {:?}",
                    group_id, e
                )
            })?
            .expect("FIXME: Client list view should exist");

        for (client_id, client_view) in client_list.into_inner().iter() {
            if let Some(client_group_id) = &client_view.group_id {
                if client_group_id == &group_id {
                    let command = ClientCommand::RemoveClientFromGroup;

                    self.client_handler
                        .execute(client_id, command)
                        .await
                        .map_err(|e| {
                            format!(
                                "Failed to remove group {} from client {} while deleting group: {:?}",
                                group_id, client_id, e
                            )
                        })?;
                }
            }
        }

        let command = GroupCommand::DeleteGroup {
            id: group_id.clone(),
        };

        match self.group_handler.execute(&group_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to delete group: {:?}", e)),
        }
    }

    pub async fn add_client_to_group(
        &self,
        group_id: String,
        client_id: String,
    ) -> Result<bool, String> {
        println!("Adding client {} to group {}", client_id, group_id);
        let command = GroupCommand::AddClientToGroup {
            client_id: client_id.clone(),
        };

        println!("Executing command to add client to group");

        match self.group_handler.execute(&group_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!(
                "Failed to add client {} to group {}: {:?}",
                client_id, group_id, e
            )),
        }?;

        println!("Executing command to assign group to client");

        let command = ClientCommand::AssignClientToGroup {
            group_id: group_id.clone(),
        };

        println!("Command created, executing...");

        match self.client_handler.execute(&client_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!(
                "Failed to assign group {} to client {}: {:?}",
                group_id, client_id, e
            )),
        }
    }

    pub async fn remove_client_from_group(
        &self,
        group_id: String,
        client_id: String,
    ) -> Result<bool, String> {
        let command = GroupCommand::RemoveClientFromGroup {
            client_id: client_id.clone(),
        };

        match self.group_handler.execute(&group_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!(
                "Failed to remove client {} from group {}: {:?}",
                client_id, group_id, e
            )),
        }?;

        let command = ClientCommand::RemoveClientFromGroup;

        match self.client_handler.execute(&client_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!(
                "Failed to remove group {} from client {}: {:?}",
                group_id, client_id, e
            )),
        }
    }

    pub async fn register_client(
        &self,
        client_id: String,
        name: String,
        wallet_address: String,
    ) -> Result<bool, String> {
        let command = ClientCommand::RegisterClient {
            id: client_id.clone(),
            name,
            wallet_address,
        };

        match self.client_handler.execute(&client_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to register client: {:?}", e)),
        }
    }

    pub async fn remove_client(&self, client_id: String) -> Result<bool, String> {
        let client = self
            .client_view
            .load(&client_id)
            .await
            .map_err(|e| {
                format!(
                    "Failed to load client view while removing client {}: {:?}",
                    client_id, e
                )
            })?
            .expect("FIXME: Client view should exist");

        if let Some(group_id) = &client.group_id {
            let command = GroupCommand::RemoveClientFromGroup {
                client_id: client_id.clone(),
            };

            self.group_handler
                .execute(&group_id, command)
                .await
                .map_err(|e| {
                    format!(
                        "Failed to remove client {} from group {} while removing client: {:?}",
                        client_id, group_id, e
                    )
                })?;
        }

        let command = ClientCommand::RemoveClient {
            id: client_id.clone(),
        };

        match self.client_handler.execute(&client_id, command).await {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to remove client: {:?}", e)),
        }
    }
}
