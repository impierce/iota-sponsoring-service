use anyhow::Result;
use anyhow::anyhow;
use application::{
    services::{
        allocation_service::AllocationService,
        balance_management_service::{BalanceManagementService, ClientRegistration},
    },
    views::sponsor_wallet::SPONSOR_WALLET_VIEW_ID,
};
use async_graphql::{InputObject, Object};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use mongo_es::MongoEventRepository;
use std::sync::Arc;
use tracing::instrument;
use url::Url;
use uuid::Uuid;
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::operations::SponsorWalletDto;
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
    /// Updates the sponsor wallet's name
    #[instrument(skip(self))]
    async fn update_sponsor_wallet(
        &self,
        input: UpdateSponsorWalletInput,
    ) -> Result<SponsorWalletDto> {
        if let Some(name) = input.name {
            self.allocation_service
                .update_sponsor_wallet_name(SPONSOR_WALLET_VIEW_ID.to_string(), name)
                .await?;
        }

        if let Some(logo_uri) = input.logo_uri {
            self.allocation_service
                .update_sponsor_wallet_logo_uri(SPONSOR_WALLET_VIEW_ID.to_string(), logo_uri)
                .await?;
        }

        self.allocation_service
            .get_sponsor_wallet_view(SPONSOR_WALLET_VIEW_ID.to_string())
            .await
            .map(SponsorWalletDto::from)
    }

    /// Creates a new group
    #[instrument(skip(self))]
    async fn create_group(&self, name: String, logo_uri: Option<Url>) -> Result<GroupDto> {
        let group_id = Uuid::new_v4();
        self.balance_management_service
            .create_group(group_id, name, logo_uri)
            .await
            .map(GroupDto::from)
    }

    /// Updates an existing group.
    #[instrument(skip(self), fields(input = ?input))]
    async fn update_group(&self, input: UpdateGroupInput) -> Result<GroupDto> {
        if let Some(name) = input.name {
            self.balance_management_service
                .update_group_name(input.group_id, name)
                .await?;
        }

        if let Some(logo_uri) = input.logo_uri {
            self.balance_management_service
                .update_group_logo_uri(input.group_id, logo_uri)
                .await?;
        }

        self.balance_management_service
            .get_group_view(input.group_id)
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

    /// Registers a new client either from a client URI or with explicit details.
    #[instrument(skip(self), fields(name = ?name, wallet_address = ?wallet_address, logo_uri = ?logo_uri, website_uri = ?website_uri, group_id = ?group_id))]
    async fn register_client(
        &self,
        name: Option<String>,
        wallet_address: Option<String>,
        logo_uri: Option<Url>,
        website_uri: Option<Url>,
        group_id: Option<Uuid>,
    ) -> Result<ClientDto> {
        let client_id = Uuid::new_v4();

        let registration = if let Some(website_uri) = website_uri {
            ClientRegistration::FromWebsiteUrl {
                website_uri,
                group_id: group_id,
            }
        } else if let (Some(name), Some(wallet_address)) = (name, wallet_address) {
            ClientRegistration::Explicit {
                name,
                wallet_address,
                logo_uri: logo_uri,
                website_uri: website_uri,
                group_id: group_id,
            }
        } else {
            return Err(anyhow!(
                "You must provide either a 'websiteUri' or both 'name' and 'walletAddress'"
            ));
        };

        self.balance_management_service
            .register_client(client_id, registration)
            .await
            .map(ClientDto::from)
    }

    /// Updates an existing client.
    #[instrument(skip(self), fields(input = ?input))]
    async fn update_client(&self, input: UpdateClientInput) -> Result<ClientDto> {
        if let Some(name) = input.name {
            self.balance_management_service
                .update_client_name(input.client_id, name)
                .await?;
        }

        if let Some(logo_uri) = input.logo_uri {
            self.balance_management_service
                .update_client_logo_uri(input.client_id, logo_uri)
                .await?;
        }

        if let Some(website_uri) = input.website_uri {
            self.balance_management_service
                .update_client_website_uri(input.client_id, website_uri)
                .await?;
        }

        if let Some(wallet_address) = input.wallet_address {
            self.balance_management_service
                .update_client_wallet_address(input.client_id, wallet_address)
                .await?;
        }

        self.balance_management_service
            .get_client_view(input.client_id)
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

#[derive(Debug, InputObject)]
struct UpdateSponsorWalletInput {
    /// The new name for the sponsor wallet.
    name: Option<String>,
    /// The new logo URI for the sponsor wallet.
    logo_uri: Option<Option<Url>>,
}

#[derive(Debug, InputObject)]
struct UpdateGroupInput {
    /// The ID of the group to update.
    group_id: Uuid,
    /// The new name for the group.
    name: Option<String>,
    /// The new logo URI for the group.
    logo_uri: Option<Option<Url>>,
}

#[derive(Debug, InputObject)]
struct UpdateClientInput {
    /// The ID of the client to update.
    client_id: Uuid,
    /// The new name for the client.
    name: Option<String>,
    /// The new logo URI for the client.
    logo_uri: Option<Option<Url>>,
    /// The new website URI for the client.
    website_uri: Option<Option<Url>>,
    /// The new wallet address for the client.
    wallet_address: Option<String>,
}
