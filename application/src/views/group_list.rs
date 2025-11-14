use std::collections::HashMap;

use balance_management::group::aggregate::Group;
use cqrs_es::{EventEnvelope, View};
use serde::{Deserialize, Serialize};

use crate::views::group::GroupView;

pub const GROUP_LIST_VIEW_ID: &str = "group_list";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupListView(HashMap<String, GroupView>);

impl GroupListView {
    pub fn into_inner(self) -> HashMap<String, GroupView> {
        self.0
    }
}

impl View<Group> for GroupListView {
    fn update(&mut self, event: &EventEnvelope<Group>) {
        self.0
            .entry(event.aggregate_id.clone())
            // or insert a new one if it doesn't exist
            .or_default()
            // update the view with the event
            .update(event);
    }
}
