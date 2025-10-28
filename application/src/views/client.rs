use balance_management::client::aggregate::Client;
use cqrs_es::{Aggregate, EventEnvelope, View};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct ClientView(Client);

impl ClientView {
    pub fn into_inner(self) -> Client {
        self.0
    }
}

impl std::ops::Deref for ClientView {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ClientView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl View<Client> for ClientView {
    fn update(&mut self, event: &EventEnvelope<Client>) {
        self.0.apply(event.payload.clone());
    }
}
