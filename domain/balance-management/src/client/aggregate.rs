use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};

use super::{
    command::ClientCommand,
    error::ClientError,
    event::ClientEvent::{self, *},
};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct Client {
    /// The unique identifier for the client.
    pub id: String,
    /// A user-friendly name for the client.
    pub name: String,
    /// The client's IOTA wallet address for transactions.
    pub wallet_address: String,
    /// An optional individual balance. This will be `Some(amount)` if the client
    /// has a dedicated balance, and `None` if they are part of a group.
    pub balance: Option<u64>,
    /// An optional ID linking the client to a group. This will be `Some(group_id)`
    /// if the client is a member of a group, and `None` otherwise.
    pub group_id: Option<String>,
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

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use ClientCommand::*;

        match command {
            RegisterClient {
                id,
                name,
                wallet_address,
            } => Ok(vec![ClientRegistered {
                id,
                name,
                wallet_address,
            }]),
            RemoveClient { id } => Ok(vec![ClientRemoved {
                id,
                group_id: self.group_id.clone(),
                is_deleted: true,
            }]),
            AssignClientToGroup { group_id } => Ok(vec![ClientAssignedToGroup {
                id: self.id.clone(),
                group_id,
            }]),
            RemoveClientFromGroup => Ok(vec![ClientRemovedFromGroup {
                id: self.id.clone(),
                group_id: None,
            }]),
            AllocateBalanceToClient { amount } => Ok(vec![BalanceAllocatedToClient {
                id: self.id.clone(),
                amount,
            }]),
            WithdrawBalanceFromClient { amount } => Ok(vec![BalanceWithdrawnFromClient {
                id: self.id.clone(),
                amount,
            }]),
        }
    }

    fn apply(&mut self, event: Self::Event) {
        use ClientEvent::*;

        match event {
            ClientRegistered {
                id,
                name,
                wallet_address,
            } => {
                self.id = id;
                self.name = name;
                self.wallet_address = wallet_address;
            }
            ClientRemoved {
                id: _,
                group_id: _,
                is_deleted,
            } => {
                *self = Self::default();
                self.is_deleted = is_deleted;
            }
            ClientAssignedToGroup { id: _, group_id } => {
                self.group_id = Some(group_id);
            }
            ClientRemovedFromGroup { id: _, group_id } => {
                self.group_id = group_id;
            }
            _ => todo!(),
        }
    }
}
