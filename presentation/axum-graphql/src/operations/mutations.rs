use anyhow::Result;
use application::services::balance_management_service::BalanceManagementService;
use async_graphql::Object;
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use mongo_es::MongoEventRepository;
use std::sync::Arc;
use uuid::Uuid;

use crate::operations::{ClientDto, GroupDto};

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
    async fn create_group(&self, group_id: Uuid, name: String) -> Result<GroupDto> {
        self.balance_management_service
            .create_group(group_id, name)
            .await
            .map(GroupDto::from)
    }

    /// Deletes a group
    async fn delete_group(&self, group_id: Uuid) -> Result<Uuid> {
        self.balance_management_service.delete_group(group_id).await
    }

    /// Adds a client to a group
    async fn add_client_to_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupDto> {
        self.balance_management_service
            .add_client_to_group(group_id, client_id)
            .await
            .map(GroupDto::from)
    }

    /// Removes a client from a group
    async fn remove_client_from_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupDto> {
        self.balance_management_service
            .remove_client_from_group(group_id, client_id)
            .await
            .map(GroupDto::from)
    }

    /// Registers a new client
    async fn register_client(
        &self,
        client_id: Uuid,
        name: String,
        wallet_address: String,
    ) -> Result<ClientDto> {
        self.balance_management_service
            .register_client(client_id, name, wallet_address)
            .await
            .map(ClientDto::from)
    }

    /// Removes a client
    async fn remove_client(&self, client_id: Uuid) -> Result<Uuid> {
        self.balance_management_service
            .remove_client(client_id)
            .await
    }
}
