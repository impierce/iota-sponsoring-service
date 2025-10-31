use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum ClientEvent {
    ClientRegistered {
        client_id: Uuid,
        name: String,
        wallet_address: String,
    },
    ClientRemoved {
        client_id: Uuid,
        group_id: Option<Uuid>,
        is_deleted: bool,
    },
    ClientAssignedToGroup {
        client_id: Uuid,
        group_id: Uuid,
    },
    ClientRemovedFromGroup {
        client_id: Uuid,
        group_id: Option<Uuid>,
    },
    BalanceAllocatedToClient {
        client_id: Uuid,
        amount: u64,
    },
    BalanceWithdrawnFromClient {
        client_id: Uuid,
        amount: u64,
    },
    ClientBalanceDecremented {
        client_id: Uuid,
        fee_paid: u64,
    },
    ClientBalanceRanLow {
        client_id: Uuid,
        current_balance: u64,
    },
}

impl DomainEvent for ClientEvent {
    fn event_type(&self) -> String {
        self.to_string()
    }

    fn event_version(&self) -> String {
        "1".to_string()
    }
}
