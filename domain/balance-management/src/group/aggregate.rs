use std::collections::HashSet;

use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace};
use uuid::Uuid;

use super::{
    command::GroupCommand,
    error::GroupError,
    event::GroupEvent::{self, *},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Group {
    /// The unique identifier for the group.
    pub group_id: Uuid,
    /// A user-friendly name for the group.
    pub name: String,
    /// The shared balance allocated to all members of this group.
    pub balance: u64,
    /// A collection of unique client IDs that are members of this group.
    pub members: HashSet<Uuid>,
    /// Indicates whether the group has been deleted.
    pub is_deleted: bool,
}

#[async_trait]
impl Aggregate for Group {
    type Command = GroupCommand;
    type Event = GroupEvent;
    type Error = GroupError;
    type Services = ();

    fn aggregate_type() -> String {
        "group".to_string()
    }

    #[instrument(name = "Group::handle", skip(self, _service), fields(command = ?command))]
    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use GroupCommand::*;
        use GroupError::*;

        debug!("Handling command");

        // --- Validation before processing the command ---
        let has_been_created = !self.group_id.is_nil();

        match command {
            CreateGroup { group_id, name } => {
                if has_been_created {
                    debug!("Validation failed: Group already exists");
                    return Err(GroupAlreadyExists);
                }
                Ok(vec![GroupCreated { group_id, name }])
            }

            // For all other commands, the group must exist first.
            _ if !has_been_created => {
                debug!("Validation failed: Group not found");
                Err(GroupNotFound)
            }

            DeleteGroup { group_id } => Ok(vec![GroupDeleted {
                group_id,
                is_deleted: true,
            }]),
            AddClientToGroup { client_id } => Ok(vec![ClientAddedToGroup {
                group_id: self.group_id.clone(),
                client_id,
            }]),
            RemoveClientFromGroup { client_id } => Ok(vec![ClientRemovedFromGroup {
                group_id: self.group_id.clone(),
                client_id,
            }]),
            AllocateFundsToGroup { group_id, amount } => {
                Ok(vec![FundsAllocatedToGroup { group_id, amount }])
            }
            WithdrawFundsFromGroup { group_id, amount } => {
                if self.balance < amount {
                    debug!("Validation failed: Insufficient balance for withdrawal");
                    return Err(InsufficientBalance);
                }
                Ok(vec![FundsWithdrawnFromGroup { group_id, amount }])
            }
            RecordTransactionFeePaid { transaction_fee } => {
                if self.balance < transaction_fee {
                    debug!("Validation failed: Insufficient balance to pay transaction fee");
                    return Err(InsufficientBalance);
                }
                let new_balance = self.balance - transaction_fee;

                Ok(vec![TransactionFeePaidRecorded {
                    group_id: self.group_id,
                    new_balance,
                }])
            }
        }
    }

    #[instrument(name = "Group::apply", skip(self), fields(event = ?event))]
    fn apply(&mut self, event: Self::Event) {
        trace!("Applying event");
        use GroupEvent::*;

        match event {
            GroupCreated { group_id, name } => {
                self.group_id = group_id;
                self.name = name;
                self.balance = 0;
                self.members = HashSet::new();
            }
            GroupDeleted {
                group_id: _,
                is_deleted,
            } => {
                *self = Self::default();
                self.is_deleted = is_deleted;
            }
            ClientAddedToGroup {
                group_id: _,
                client_id,
            } => {
                self.members.insert(client_id);
            }
            ClientRemovedFromGroup {
                group_id: _,
                client_id,
            } => {
                self.members.remove(&client_id);
            }
            FundsAllocatedToGroup {
                group_id: _,
                amount,
            } => {
                self.balance = self.balance.saturating_add(amount);
            }
            FundsWithdrawnFromGroup {
                group_id: _,
                amount,
            } => {
                self.balance = self.balance.saturating_sub(amount);
            }
            TransactionFeePaidRecorded {
                group_id: _,
                new_balance,
            } => {
                self.balance = new_balance;
            }
            _ => unimplemented!(),
        }
    }
}
