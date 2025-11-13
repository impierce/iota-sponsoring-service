use balance_management::group::{aggregate::Group, event::GroupEvent};
use chrono::{DateTime, Utc};
use cqrs_es::{EventEnvelope, View};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SponsorshipTransactionView {
    #[serde(default)]
    pub sponsorship_transaction_id: Uuid,
    pub group_id: Uuid,
    pub client_id: Uuid,
    #[serde(default)]
    pub client_name: String,
    pub transaction_fee: u64,
    #[serde(default)]
    pub transaction_fee_iot: f64,
    pub transaction_fee_eur: f64,
    pub transaction_fee_usd: f64,
    pub timestamp: DateTime<Utc>,
}

impl View<Group> for SponsorshipTransactionView {
    fn update(&mut self, event: &EventEnvelope<Group>) {
        use GroupEvent::*;

        match &event.payload {
            TransactionFeePaidRecorded {
                sponsorship_transaction_id,
                group_id,
                client_id,
                client_name,
                transaction_fee,
                transaction_fee_iot,
                transaction_fee_eur,
                transaction_fee_usd,
                new_balance: _,
                timestamp,
            } => {
                self.sponsorship_transaction_id = *sponsorship_transaction_id;
                self.group_id = group_id.clone();
                self.client_id = client_id.clone();
                self.client_name = client_name.clone();
                self.transaction_fee = *transaction_fee;
                self.transaction_fee_iot = *transaction_fee_iot;
                self.transaction_fee_eur = *transaction_fee_eur;
                self.transaction_fee_usd = *transaction_fee_usd;
                self.timestamp = *timestamp;
            }
            _ => {}
        }
    }
}
