use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace};
use url::Url;
use uuid::Uuid;

use super::{
    command::ClientCommand,
    error::ClientError,
    event::ClientEvent::{self, *},
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct Client {
    /// The unique identifier for the client.
    pub client_id: Uuid,
    /// A user-friendly name for the client.
    pub name: String,
    /// Logo URI for the client.
    pub logo_uri: Option<Url>,
    /// Website URI for more information about the client.
    pub website_uri: Option<Url>,
    /// The client's IOTA wallet address for transactions.
    pub wallet_address: String,
    /// An optional individual balance. This will be `Some(amount)` if the client
    /// has a dedicated balance, and `None` if they are part of a group.
    pub balance: Option<u64>,
    /// An optional ID linking the client to a group. This will be `Some(group_id)`
    /// if the client is a member of a group, and `None` otherwise.
    pub group_id: Option<Uuid>,
    /// Indicates whether the client has been deleted.
    pub is_deleted: bool,
}

#[async_trait]
impl Aggregate for Client {
    type Command = ClientCommand;
    type Event = ClientEvent;
    type Error = ClientError;
    type Services = ();

    fn aggregate_type() -> String {
        "client".to_string()
    }

    #[instrument(name = "Client::handle", skip(self, _service), fields(command = ?command))]
    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use ClientCommand::*;
        use ClientError::*;

        debug!("Handling command");

        // --- Validation before processing the command ---
        let has_been_created = !self.client_id.is_nil();

        match command {
            RegisterClient {
                client_id,
                name,
                logo_uri,
                website_uri,
                wallet_address,
            } => {
                if has_been_created {
                    debug!("Validation failed: Client already exists");
                    return Err(ClientAlreadyExists);
                }
                Ok(vec![ClientRegistered {
                    client_id,
                    name,
                    logo_uri,
                    website_uri,
                    wallet_address,
                }])
            }

            // For all other commands, the client must exist first.
            _ if !has_been_created => {
                debug!("Validation failed: Client not found");
                Err(ClientNotFound)
            }

            UpdateClientName { name } => Ok(vec![ClientNameUpdated { name }]),
            UpdateClientLogoUri { logo_uri } => Ok(vec![ClientLogoUriUpdated { logo_uri }]),
            UpdateClientWebsiteUri { website_uri } => {
                Ok(vec![ClientWebsiteUriUpdated { website_uri }])
            }
            UpdateClientWalletAddress { wallet_address } => {
                Ok(vec![ClientWalletAddressUpdated { wallet_address }])
            }

            RemoveClient { client_id } => Ok(vec![ClientRemoved {
                client_id,
                group_id: self.group_id.clone(),
                is_deleted: true,
            }]),
            AssignClientToGroup { group_id } => Ok(vec![ClientAssignedToGroup {
                client_id: self.client_id.clone(),
                group_id,
            }]),
            RemoveClientFromGroup => Ok(vec![ClientRemovedFromGroup {
                client_id: self.client_id.clone(),
                group_id: None,
            }]),
            AllocateBalanceToClient { amount } => Ok(vec![BalanceAllocatedToClient {
                client_id: self.client_id.clone(),
                amount,
            }]),
            WithdrawBalanceFromClient { amount } => {
                if self.balance.unwrap_or(0) < amount {
                    debug!("Validation failed: Insufficient balance for withdrawal");
                    return Err(InsufficientBalance);
                }
                Ok(vec![BalanceWithdrawnFromClient {
                    client_id: self.client_id,
                    amount,
                }])
            }
        }
    }

    #[instrument(name = "Client::apply", skip(self), fields(event = ?event))]
    fn apply(&mut self, event: Self::Event) {
        trace!("Applying event");
        use ClientEvent::*;

        match event {
            ClientRegistered {
                client_id,
                name,
                logo_uri,
                website_uri,
                wallet_address,
            } => {
                self.client_id = client_id;
                self.name = name;
                self.logo_uri = logo_uri;
                self.website_uri = website_uri;
                self.wallet_address = wallet_address;
            }
            ClientNameUpdated { name } => {
                self.name = name;
            }
            ClientLogoUriUpdated { logo_uri } => {
                self.logo_uri = logo_uri;
            }
            ClientWebsiteUriUpdated { website_uri } => {
                self.website_uri = website_uri;
            }
            ClientWalletAddressUpdated { wallet_address } => {
                self.wallet_address = wallet_address;
            }
            ClientRemoved {
                client_id: _,
                group_id: _,
                is_deleted,
            } => {
                *self = Self::default();
                self.is_deleted = is_deleted;
            }
            ClientAssignedToGroup {
                client_id: _,
                group_id,
            } => {
                self.group_id = Some(group_id);
            }
            ClientRemovedFromGroup {
                client_id: _,
                group_id,
            } => {
                self.group_id = group_id;
            }
            _ => todo!(),
        }
    }
}
