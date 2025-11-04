use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum GroupEvent {
    GroupCreated {
        group_id: Uuid,
        name: String,
    },
    GroupDeleted {
        group_id: Uuid,
        is_deleted: bool,
    },
    ClientAddedToGroup {
        group_id: Uuid,
        client_id: Uuid,
    },
    ClientRemovedFromGroup {
        group_id: Uuid,
        client_id: Uuid,
    },
    FundsAllocatedToGroup {
        group_id: Uuid,
        amount: u64,
    },
    FundsWithdrawnFromGroup {
        group_id: Uuid,
        amount: u64,
    },
    GroupBalanceDecremented {
        group_id: Uuid,
        by_client_id: String,
        fee_paid: u64,
    },
    TransactionFeePaidRecorded {
        group_id: Uuid,
        new_balance: u64,
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
