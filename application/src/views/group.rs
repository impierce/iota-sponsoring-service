use balance_management::group::aggregate::Group;
use cqrs_es::{Aggregate, EventEnvelope, View};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupView(Group);

impl GroupView {
    pub fn into_inner(self) -> Group {
        self.0
    }
}

impl std::ops::Deref for GroupView {
    type Target = Group;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for GroupView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl View<Group> for GroupView {
    fn update(&mut self, event: &EventEnvelope<Group>) {
        self.0.apply(event.payload.clone());
    }
}
