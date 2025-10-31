use std::collections::HashSet;

use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};

use super::{
    command::GroupCommand,
    error::GroupError,
    event::GroupEvent::{self, *},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Group {
    /// The unique identifier for the group.
    pub id: String,
    /// A user-friendly name for the group.
    pub name: String,
    /// The shared balance allocated to all members of this group.
    pub balance: u64,
    /// A collection of unique client IDs that are members of this group.
    pub members: HashSet<String>,
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

        match command {
            CreateGroup { id, name } => Ok(vec![GroupCreated { id, name }]),
            DeleteGroup { id } => Ok(vec![GroupDeleted {
                id,
                is_deleted: true,
            }]),
            AddClientToGroup { client_id } => Ok(vec![ClientAddedToGroup {
                id: self.id.clone(),
                client_id,
            }]),
            RemoveClientFromGroup { client_id } => Ok(vec![ClientRemovedFromGroup {
                id: self.id.clone(),
                client_id,
            }]),
            AllocateBalanceToGroup { id, amount } => {
                Ok(vec![BalanceAllocatedToGroup { id, amount }])
            }
            WithdrawBalanceFromGroup { id, amount } => {
                Ok(vec![BalanceWithdrawnFromGroup { id, amount }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        use GroupEvent::*;

        match event {
            GroupCreated { id, name } => {
                self.id = id;
                self.name = name;
                self.balance = 0;
                self.members = HashSet::new();
            }
            GroupDeleted { id: _, is_deleted } => {
                *self = Self::default();
                self.is_deleted = is_deleted;
            }
            ClientAddedToGroup { id: _, client_id } => {
                self.members.insert(client_id);
            }
            ClientRemovedFromGroup { id: _, client_id } => {
                self.members.remove(&client_id);
            }
            _ => todo!(),
        }
    }
}
