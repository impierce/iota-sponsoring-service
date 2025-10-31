use std::collections::HashSet;

use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};
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

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use GroupCommand::*;
        use GroupError::*;

        // --- Validation before processing the command ---
        let has_been_created = !self.group_id.is_nil();

        match command {
            CreateGroup { group_id, name } => Ok(vec![GroupCreated { group_id, name }]),

            // For all other commands, the deck must exist first.
            _ if !has_been_created => Err(GroupNotFound),

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
            AllocateBalanceToGroup { group_id, amount } => {
                Ok(vec![BalanceAllocatedToGroup { group_id, amount }])
            }
            WithdrawBalanceFromGroup { group_id, amount } => {
                Ok(vec![BalanceWithdrawnFromGroup { group_id, amount }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
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
            _ => todo!(),
        }
    }
}
