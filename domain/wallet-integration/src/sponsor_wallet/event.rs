use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum SponsorWalletEvent {
    SponsorWalletCreated {
        sponsor_wallet_id: String,
        address: String,
        balance: u64,
    },
    BalanceUpdateRecorded {
        new_balance: u64,
    },
    TransactionFeePaidRecorded {
        sender_address: String,
        new_balance: u64,
    },
}

impl DomainEvent for SponsorWalletEvent {
    fn event_type(&self) -> String {
        self.to_string()
    }

    fn event_version(&self) -> String {
        "1".to_string()
    }
}
