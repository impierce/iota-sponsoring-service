use std::collections::HashMap;

use balance_management::group::aggregate::Group;
use cqrs_es::{EventEnvelope, View};
use serde::{Deserialize, Serialize};

use crate::views::sponsorship_transaction::SponsorshipTransactionView;

pub const SPONSORSHIP_TRANSACTION_LIST_VIEW_ID: &str = "sponsorship_transaction_list";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SponsorshipTransactionListView(HashMap<String, SponsorshipTransactionView>);

impl SponsorshipTransactionListView {
    pub fn into_inner(self) -> HashMap<String, SponsorshipTransactionView> {
        self.0
    }
}

impl View<Group> for SponsorshipTransactionListView {
    fn update(&mut self, event: &EventEnvelope<Group>) {
        use balance_management::group::event::GroupEvent::*;

        match &event.payload {
            TransactionFeePaidRecorded {
                sponsorship_transaction_id,
                ..
            } => {
                let key = sponsorship_transaction_id.to_string();

                self.0
                    .entry(key)
                    // or insert a new one if it doesn't exist
                    .or_default()
                    // update the view with the event
                    .update(event);
            }
            _ => {}
        }
    }
}
