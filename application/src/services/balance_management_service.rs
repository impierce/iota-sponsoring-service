use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use serde::Deserialize;
use tracing::{debug, info, instrument, warn};
use url::Url;
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

    #[instrument(skip(self), fields(group_id = %group_id, name = %name, logo_uri = ?logo_uri))]
    pub async fn create_group(
        &self,
        group_id: Uuid,
        name: String,
        logo_uri: Option<Url>,
    ) -> Result<GroupView> {
        info!("Creating new group");
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
            logo_uri,
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
    pub async fn get_group_view(&self, group_id: Uuid) -> Result<GroupView> {
        info!("Loading group view");
        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Group view not found for id `{}`", group_id))?;

        info!("Successfully loaded group view");
        Ok(view)
    }

    #[instrument(skip(self), fields(group_id = %group_id, name = %name))]
    pub async fn update_group_name(&self, group_id: Uuid, name: String) -> Result<GroupView> {
        info!("Updating group name");
        let command = GroupCommand::UpdateGroupName { name };

        debug!("Dispatching `UpdateGroupName` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after updating name for group `{}`",
                    group_id
                )
            })?;

        info!("Successfully updated group name");
        Ok(view)
    }

    #[instrument(skip(self), fields(group_id = %group_id, logo_uri = ?logo_uri))]
    pub async fn update_group_logo_uri(
        &self,
        group_id: Uuid,
        logo_uri: Option<Url>,
    ) -> Result<GroupView> {
        info!("Updating group logo URI");
        let command = GroupCommand::UpdateGroupLogoUri { logo_uri };

        debug!("Dispatching `UpdateGroupLogoUri` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found after updating logo URI for group `{}`",
                    group_id
                )
            })?;

        info!("Successfully updated group logo URI");
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

    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn register_client(
        &self,
        client_id: Uuid,
        registration: ClientRegistration,
    ) -> Result<ClientView> {
        info!("Registering new client");

        let (name, wallet_address, logo_uri, website_uri, group_id) = match registration {
            ClientRegistration::Explicit {
                name,
                wallet_address,
                logo_uri,
                website_uri,
                group_id,
            } => {
                info!("Registering client with explicit details");
                (name, wallet_address, logo_uri, website_uri, group_id)
            }
            ClientRegistration::FromWebsiteUrl {
                website_uri,
                group_id,
            } => {
                info!(%website_uri, "Registering client from website_uri");

                let IotaGasStationClientData {
                    name,
                    logo_uri,
                    iota_address: wallet_address,
                } = reqwest::get(
                    website_uri
                        .join("/.well-known/sponsoring-configuration")
                        .unwrap()
                        .as_str(),
                )
                .await
                .map_err(|e| anyhow!("Failed to fetch client info from `{website_uri}`: {e}"))?
                .json::<IotaGasStationClientData>()
                .await
                .map_err(|e| {
                    anyhow!("Failed to parse client info from `{website_uri}` response: {e}")
                })?;

                (name, wallet_address, logo_uri, Some(website_uri), group_id)
            }
        };

        if let Some(group_id) = group_id {
            self.group_view
                .load(&group_id.to_string())
                .await?
                .ok_or_else(|| {
                    anyhow!(
                        "Group view not found when registering client `{}` to group `{}`",
                        client_id,
                        group_id
                    )
                })?;
        }

        let command = ClientCommand::RegisterClient {
            client_id,
            name,
            wallet_address,
            logo_uri,
            website_uri,
        };

        debug!("Dispatching `RegisterClient` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        if let Some(group_id) = group_id {
            self.add_client_to_group(group_id, client_id).await?;
        }

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found after registering client `{}`",
                    client_id
                )
            })?;

        info!("Successfully registered client");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id))]
    pub async fn get_client_view(&self, client_id: Uuid) -> Result<ClientView> {
        info!("Loading client view");
        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| anyhow!("Client view not found for id `{}`", client_id))?;

        info!("Successfully loaded client view");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id, name = %name))]
    pub async fn update_client_name(&self, client_id: Uuid, name: String) -> Result<ClientView> {
        info!("Updating client name");
        let command = ClientCommand::UpdateClientName { name };

        debug!("Dispatching `UpdateClientName` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found after updating name for client `{}`",
                    client_id
                )
            })?;

        info!("Successfully updated client name");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id, logo_uri = ?logo_uri))]
    pub async fn update_client_logo_uri(
        &self,
        client_id: Uuid,
        logo_uri: Option<Url>,
    ) -> Result<ClientView> {
        info!("Updating client logo URI");
        let command = ClientCommand::UpdateClientLogoUri { logo_uri };

        debug!("Dispatching `UpdateClientLogoUri` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found after updating logo URI for client `{}`",
                    client_id
                )
            })?;

        info!("Successfully updated client logo URI");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id, website_uri = ?website_uri))]
    pub async fn update_client_website_uri(
        &self,
        client_id: Uuid,
        website_uri: Option<Url>,
    ) -> Result<ClientView> {
        info!("Updating client website URI");
        let command = ClientCommand::UpdateClientWebsiteUri { website_uri };

        debug!("Dispatching `UpdateClientWebsiteUri` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found after updating website URI for client `{}`",
                    client_id
                )
            })?;

        info!("Successfully updated client website URI");
        Ok(view)
    }

    #[instrument(skip(self), fields(client_id = %client_id, wallet_address = %wallet_address))]
    pub async fn update_client_wallet_address(
        &self,
        client_id: Uuid,
        wallet_address: String,
    ) -> Result<ClientView> {
        info!("Updating client wallet address");
        let command = ClientCommand::UpdateClientWalletAddress { wallet_address };

        debug!("Dispatching `UpdateClientWalletAddress` command");
        self.client_handler
            .execute(&client_id.to_string(), command)
            .await?;

        let view = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found after updating wallet address for client `{}`",
                    client_id
                )
            })?;

        info!("Successfully updated client wallet address");
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

#[derive(Deserialize)]
pub struct IotaGasStationClientData {
    pub name: String,
    pub logo_uri: Option<Url>,
    pub iota_address: String,
}

/// Defines the information needed to register a new client.
#[derive(Debug)]
pub enum ClientRegistration {
    /// Register a client by providing their details directly.
    Explicit {
        name: String,
        wallet_address: String,
        logo_uri: Option<Url>,
        website_uri: Option<Url>,
        group_id: Option<Uuid>,
    },
    /// Register a client by discovering details from their `website_uri`.
    FromWebsiteUrl {
        website_uri: Url,
        group_id: Option<Uuid>,
    },
}
