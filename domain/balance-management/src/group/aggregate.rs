use std::collections::HashSet;

use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace};
use url::Url;
use uuid::Uuid;

use super::{
    command::GroupCommand,
    error::GroupError,
    event::GroupEvent::{self, *},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Action,
    Warning,
    #[default]
    Success,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Status {
    pub variant: Variant,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Group {
    /// The unique identifier for the group.
    pub group_id: Uuid,
    /// A user-friendly name for the group.
    pub name: String,
    /// Logo URI for the group.
    pub logo_uri: Option<Url>,
    /// The shared balance allocated to all members of this group.
    pub balance: u64,
    /// A collection of unique client IDs that are members of this group.
    pub members: HashSet<Uuid>,
    /// The status indicating the current state of the group.
    pub status: Status,
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
            CreateGroup {
                group_id,
                name,
                logo_uri,
            } => {
                if has_been_created {
                    debug!("Validation failed: Group already exists");
                    return Err(GroupAlreadyExists);
                }
                Ok(vec![GroupCreated {
                    group_id,
                    name,
                    logo_uri,
                }])
            }

            // For all other commands, the group must exist first.
            _ if !has_been_created => {
                debug!("Validation failed: Group not found");
                Err(GroupNotFound)
            }

            UpdateGroupName { name } => Ok(vec![GroupNameUpdated { name }]),
            UpdateGroupLogoUri { logo_uri } => Ok(vec![GroupLogoUriUpdated { logo_uri }]),
            UpdateGroupStatus { status } => Ok(vec![GroupStatusUpdated { status }]),

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
            RecordTransactionFeePaid {
                client_id,
                client_name,
                transaction_fee,
                transaction_fee_iot,
                transaction_fee_eur,
                transaction_fee_usd,
            } => {
                // TODO: fix transaction fee deduction logic
                // if self.balance < transaction_fee {
                //     debug!("Validation failed: Insufficient balance to pay transaction fee");
                //     return Err(InsufficientBalance);
                // }
                let transaction_fee = 1_000_000;
                let new_balance = self.balance.saturating_sub(transaction_fee);

                let timestamp = chrono::Utc::now();

                let sponsorship_transaction_id = Uuid::new_v4();

                Ok(vec![TransactionFeePaidRecorded {
                    sponsorship_transaction_id,
                    group_id: self.group_id,
                    client_id,
                    client_name,
                    transaction_fee,
                    transaction_fee_iot,
                    transaction_fee_eur,
                    transaction_fee_usd,
                    new_balance,
                    timestamp,
                }])
            }
            RecordTransactionFeePaidForDemo {
                client_id,
                client_name,
                transaction_fee,
                transaction_fee_iot,
                transaction_fee_eur,
                transaction_fee_usd,
                timestamp,
            } => {
                // In demo mode, we allow recording transaction fees without checking the balance.
                let new_balance = self.balance;

                let sponsorship_transaction_id = Uuid::new_v4();

                Ok(vec![TransactionFeePaidRecorded {
                    sponsorship_transaction_id,
                    group_id: self.group_id,
                    client_id,
                    client_name,
                    transaction_fee,
                    transaction_fee_iot,
                    transaction_fee_eur,
                    transaction_fee_usd,
                    new_balance,
                    timestamp,
                }])
            }
        }
    }

    #[instrument(name = "Group::apply", skip(self), fields(event = ?event))]
    fn apply(&mut self, event: Self::Event) {
        trace!("Applying event");
        use GroupEvent::*;

        match event {
            GroupCreated {
                group_id,
                name,
                logo_uri,
            } => {
                self.group_id = group_id;
                self.name = name;
                self.logo_uri = logo_uri;
                self.balance = 0;
                self.members = HashSet::new();
            }
            GroupNameUpdated { name } => {
                self.name = name;
            }
            GroupLogoUriUpdated { logo_uri } => {
                self.logo_uri = logo_uri;
            }
            GroupStatusUpdated { status } => {
                self.status = status;
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
                sponsorship_transaction_id: _,
                group_id: _,
                client_id: _,
                client_name: _,
                transaction_fee: _,
                transaction_fee_iot: _,
                transaction_fee_eur: _,
                transaction_fee_usd: _,
                new_balance,
                timestamp: _,
            } => {
                self.balance = new_balance;
            }
        }
    }
}
