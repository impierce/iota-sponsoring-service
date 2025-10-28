use application::services::balance_management_service::BalanceManagementService;
use async_graphql::Object;
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use mongo_es::MongoEventRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct MutationRoot {
    balance_management_service: Arc<
        BalanceManagementService<
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
}

impl MutationRoot {
    pub fn new(
        balance_management_service: Arc<
            BalanceManagementService<
                PersistedEventStore<MongoEventRepository, Client>,
                PersistedEventStore<MongoEventRepository, Group>,
            >,
        >,
    ) -> Self {
        Self {
            balance_management_service,
        }
    }
}

#[Object]
impl MutationRoot {
    /// Creates a new group
    async fn create_group(&self, group_id: String, name: String) -> Result<bool, String> {
        self.balance_management_service
            .create_group(group_id, name)
            .await
    }

    /// Deletes a group
    async fn delete_group(&self, group_id: String) -> Result<bool, String> {
        self.balance_management_service.delete_group(group_id).await
    }

    /// Adds a client to a group
    async fn add_client_to_group(
        &self,
        group_id: String,
        client_id: String,
    ) -> Result<bool, String> {
        self.balance_management_service
            .add_client_to_group(group_id, client_id)
            .await
    }

    /// Removes a client from a group
    async fn remove_client_from_group(
        &self,
        group_id: String,
        client_id: String,
    ) -> Result<bool, String> {
        self.balance_management_service
            .remove_client_from_group(group_id, client_id)
            .await
    }

    /// Registers a new client
    async fn register_client(
        &self,
        client_id: String,
        name: String,
        wallet_address: String,
    ) -> Result<bool, String> {
        self.balance_management_service
            .register_client(client_id, name, wallet_address)
            .await
    }

    /// Removes a client
    async fn remove_client(&self, client_id: String) -> Result<bool, String> {
        self.balance_management_service
            .remove_client(client_id)
            .await
    }
}
