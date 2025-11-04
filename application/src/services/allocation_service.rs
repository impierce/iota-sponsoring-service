use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::{self, aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use uuid::Uuid;
use wallet_integration::sponsor_wallet::{
    self, aggregate::SponsorWallet, command::SponsorWalletCommand,
};

use crate::views::{
    client::ClientView,
    client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
    group::GroupView,
    group_list::{self, GROUP_LIST_VIEW_ID, GroupListView},
    sponsor_wallet::SponsorWalletView,
};
pub struct AllocationService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet>,
    CES: EventStore<Client>,
    GES: EventStore<Group>,
{
    sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
    client_handler: Arc<CqrsFramework<Client, CES>>,
    group_handler: Arc<CqrsFramework<Group, GES>>,
    sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
    client_view: Arc<dyn ViewRepository<ClientView, Client>>,
    client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
    group_view: Arc<dyn ViewRepository<GroupView, Group>>,
    group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
}

impl<SWES, CES, GES> AllocationService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet> + 'static,
    CES: EventStore<Client> + 'static,
    GES: EventStore<Group> + 'static,
{
    pub fn new(
        sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
        client_handler: Arc<CqrsFramework<Client, CES>>,
        group_handler: Arc<CqrsFramework<Group, GES>>,
        sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
        client_view: Arc<dyn ViewRepository<ClientView, Client>>,
        client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
        group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            sponsor_wallet_handler,
            client_handler,
            group_handler,
            sponsor_wallet_view,
            client_view,
            client_list_view,
            group_view,
            group_list_view,
        }
    }

    pub async fn record_balance_update(
        &self,
        sponsor_wallet_id: String,
        new_balance: u64,
    ) -> Result<SponsorWalletView> {
        println!(
            "Recording balance update for sponsor wallet `{}` with new balance {}",
            sponsor_wallet_id, new_balance
        );
        let command = SponsorWalletCommand::RecordBalanceUpdate { new_balance };

        println!("Executing command...");
        self.sponsor_wallet_handler
            .execute(&sponsor_wallet_id, command)
            .await?;

        println!("Loading updated sponsor wallet view...");
        self.sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                anyhow!("Sponsor wallet view not found after recording balance update for `{sponsor_wallet_id}`")
            }).inspect(|view| {
                println!("Updated SponsorWalletView: {:?}", view);
            }).inspect_err(
                |err| println!("Error loading SponsorWalletView: {:?}", err
            ))
    }

    pub async fn record_transaction_fee_paid(
        &self,
        sender_address: String,
        transaction_fee: u64,
    ) -> Result<()> {
        println!(
            "Recording transaction fee paid for sender address `{}` with fee amount {}",
            sender_address, transaction_fee
        );

        let client_list_view = self
            .client_list_view
            .load(CLIENT_LIST_VIEW_ID)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Client list view not found for id `{}` during transaction fee recording",
                    CLIENT_LIST_VIEW_ID
                )
            })?;

        let client_view = client_list_view
            .into_inner()
            .into_values()
            .find(|client_view| client_view.wallet_address == sender_address)
            .ok_or_else(|| {
                anyhow!(
                    "Client view not found for address `{}` during transaction fee recording",
                    sender_address
                )
            })?;

        if let Some(group_id) = &client_view.group_id {
            let command = GroupCommand::RecordTransactionFeePaid { transaction_fee };

            self.group_handler
                .execute(&group_id.to_string(), command)
                .await
                .map_err(|e| anyhow!("Failed to record transaction fee paid to group: {}", e))?;
        } else {
            todo!(
                "Client with address `{}` does not belong to any group. Skipping group fee recording.",
                sender_address
            );
        }

        Ok(())
    }

    pub async fn allocate_funds_to_group(
        &self,
        sponsor_wallet_id: String,
        group_id: Uuid,
        amount: u64,
    ) -> Result<GroupView> {
        let sponsor_wallet_view = self
            .sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Sponsor wallet view not found for id `{}` during allocation",
                    sponsor_wallet_id
                )
            })?;

        let group_list_view = self
            .group_list_view
            .load(GROUP_LIST_VIEW_ID)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "Group view not found for id `{}` during allocation",
                    group_id
                )
            })?;

        let allocated_balance = group_list_view
            .into_inner()
            .into_values()
            .map(|group_view| group_view.balance)
            .sum::<u64>();

        if sponsor_wallet_view.balance < allocated_balance + amount {
            return Err(anyhow!(
                "Insufficient balance in sponsor wallet `{}` for allocation. Available: {}, Requested: {}",
                sponsor_wallet_id,
                sponsor_wallet_view.balance,
                amount
            ));
        }

        let command = GroupCommand::AllocateFundsToGroup { group_id, amount };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        self.group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!("Group view not found after allocating balance to group `{group_id}`")
            })
    }

    pub async fn withdraw_funds_from_group(
        &self,
        group_id: Uuid,
        amount: u64,
    ) -> Result<GroupView> {
        let command = GroupCommand::WithdrawFundsFromGroup { group_id, amount };

        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        self.group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!("Group view not found after withdrawing balance from group `{group_id}`")
            })
    }
}
