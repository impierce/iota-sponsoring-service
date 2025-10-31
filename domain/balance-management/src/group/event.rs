use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum GroupEvent {
    GroupCreated {
        id: String,
        name: String,
    },
    GroupDeleted {
        id: String,
        is_deleted: bool,
    },
    ClientAddedToGroup {
        id: String,
        client_id: String,
    },
    ClientRemovedFromGroup {
        id: String,
        client_id: String,
    },
    BalanceAllocatedToGroup {
        id: String,
        amount: u64,
    },
    BalanceWithdrawnFromGroup {
        id: String,
        amount: u64,
    },
    GroupBalanceDecremented {
        id: String,
        by_client_id: String,
        fee_paid: u64,
    },
}

impl DomainEvent for GroupEvent {
    fn event_type(&self) -> String {
        self.to_string()
    }

    fn event_version(&self) -> String {
        "1".to_string()
    }
}
