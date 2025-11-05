use anyhow::Result;
use application::{
    services::{
        allocation_service::AllocationService, balance_management_service::BalanceManagementService,
    },
    views::sponsor_wallet::SPONSOR_WALLET_VIEW_ID,
};
use async_graphql::Object;
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use mongo_es::MongoEventRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::operations::{ClientDto, GroupDto};

#[derive(Clone)]
pub struct MutationRoot {
    allocation_service: Arc<
        AllocationService<
            PersistedEventStore<MongoEventRepository, SponsorWallet>,
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
    balance_management_service: Arc<
        BalanceManagementService<
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
}

impl MutationRoot {
    pub fn new(
        allocation_service: Arc<
            AllocationService<
                PersistedEventStore<MongoEventRepository, SponsorWallet>,
                PersistedEventStore<MongoEventRepository, Client>,
                PersistedEventStore<MongoEventRepository, Group>,
            >,
        >,
        balance_management_service: Arc<
            BalanceManagementService<
                PersistedEventStore<MongoEventRepository, Client>,
                PersistedEventStore<MongoEventRepository, Group>,
            >,
        >,
    ) -> Self {
        Self {
            allocation_service,
            balance_management_service,
        }
    }
}

#[Object]
impl MutationRoot {
    /// Creates a new group
    #[instrument(skip(self))]
    async fn create_group(&self, name: String) -> Result<GroupDto> {
        let group_id = Uuid::new_v4();
        self.balance_management_service
            .create_group(group_id, name)
            .await
            .map(GroupDto::from)
    }

    /// Deletes a group
    #[instrument(skip(self))]
    async fn delete_group(&self, group_id: Uuid) -> Result<Uuid> {
        self.balance_management_service.delete_group(group_id).await
    }

    /// Adds a client to a group
    #[instrument(skip(self))]
    async fn add_client_to_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupDto> {
        self.balance_management_service
            .add_client_to_group(group_id, client_id)
            .await
            .map(GroupDto::from)
    }

    /// Removes a client from a group
    #[instrument(skip(self))]
    async fn remove_client_from_group(&self, group_id: Uuid, client_id: Uuid) -> Result<GroupDto> {
        self.balance_management_service
            .remove_client_from_group(group_id, client_id)
            .await
            .map(GroupDto::from)
    }

    /// Registers a new client
    #[instrument(skip(self))]
    async fn register_client(&self, name: String, wallet_address: String) -> Result<ClientDto> {
        let client_id = Uuid::new_v4();

        self.balance_management_service
            .register_client(client_id, name, wallet_address)
            .await
            .map(ClientDto::from)
    }

    /// Removes a client
    #[instrument(skip(self))]
    async fn remove_client(&self, client_id: Uuid) -> Result<Uuid> {
        self.balance_management_service
            .remove_client(client_id)
            .await
    }

    /// Allocates funds to a group
    #[instrument(skip(self))]
    async fn allocate_funds_to_group(&self, group_id: Uuid, amount: u64) -> Result<GroupDto> {
        self.allocation_service
            .allocate_funds_to_group(SPONSOR_WALLET_VIEW_ID.to_string(), group_id, amount)
            .await
            .map(GroupDto::from)
    }

    /// Withdraws funds from a group
    #[instrument(skip(self))]
    async fn withdraw_funds_from_group(&self, group_id: Uuid, amount: u64) -> Result<GroupDto> {
        self.allocation_service
            .withdraw_funds_from_group(group_id, amount)
            .await
            .map(GroupDto::from)
    }
}
