use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum ClientEvent {
    ClientRegistered {
        id: String,
        name: String,
        wallet_address: String,
    },
    ClientRemoved {
        id: String,
        group_id: Option<String>,
    },
    ClientAssignedToGroup {
        id: String,
        group_id: String,
    },
    ClientRemovedFromGroup {
        id: String,
        group_id: Option<String>,
    },
    BalanceAllocatedToClient {
        id: String,
        amount: u64,
    },
    BalanceWithdrawnFromClient {
        id: String,
        amount: u64,
    },
    ClientBalanceDecremented {
        id: String,
        fee_paid: u64,
    },
    ClientBalanceRanLow {
        id: String,
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
