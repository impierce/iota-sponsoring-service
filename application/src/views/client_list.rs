use std::collections::HashMap;

use balance_management::client::aggregate::Client;
use cqrs_es::{EventEnvelope, View};
use serde::{Deserialize, Serialize};

use crate::views::client::ClientView;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ClientListView(HashMap<String, ClientView>);

impl ClientListView {
    pub fn into_inner(self) -> HashMap<String, ClientView> {
        self.0
    }
}

impl View<Client> for ClientListView {
    fn update(&mut self, event: &EventEnvelope<Client>) {
        self.0
            .entry(event.aggregate_id.clone())
            // or insert a new one if it doesn't exist
            .or_default()
            // update the view with the event
            .update(event);
    }
}
