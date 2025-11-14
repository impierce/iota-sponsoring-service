use chrono::{DateTime, Utc};
use cqrs_es::DomainEvent;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::group::aggregate::Status;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, strum::Display)]
pub enum GroupEvent {
    GroupCreated {
        group_id: Uuid,
        name: String,
        logo_uri: Option<Url>,
    },
    GroupNameUpdated {
        name: String,
    },
    GroupLogoUriUpdated {
        logo_uri: Option<Url>,
    },
    GroupStatusUpdated {
        status: Status,
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
    TransactionFeePaidRecorded {
        #[serde(default)]
        sponsorship_transaction_id: Uuid,
        group_id: Uuid,
        client_id: Uuid,
        #[serde(default)]
        client_name: String,
        transaction_fee: u64,
        #[serde(default)]
        transaction_fee_iot: f64,
        transaction_fee_eur: f64,
        transaction_fee_usd: f64,
        new_balance: u64,
        timestamp: DateTime<Utc>,
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
