use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use base64::prelude::*;
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use iota_gas_station::access_controller::hook::ExecuteTxHookRequest;
use iota_types::transaction::{TransactionData, TransactionDataAPI};
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
pub struct AuthorizeTransactionService<SWES, CES, GES>
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

impl<SWES, CES, GES> AuthorizeTransactionService<SWES, CES, GES>
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

    pub async fn authorize_transaction(
        &self,
        transaction_data: ExecuteTxHookRequest,
    ) -> Result<()> {
        let transaction_data: TransactionData = BASE64_STANDARD
            .decode(&transaction_data.execute_tx_request.payload.tx_bytes)
            .context("failed to decode base64 string with transaction data")
            .and_then(|bytes| {
                bcs::from_bytes(&bytes).context("failed to parse BCS bytes to `TransactionData`")
            })
            .expect("FIXME: handle error properly");

        let sender_address = transaction_data.sender().to_string();
        let kind = transaction_data.kind();
        let gas_owner = transaction_data.gas_owner().to_string();
        let gas = transaction_data.gas();
        let gas_price = transaction_data.gas_price();
        let gas_budget = transaction_data.gas_budget();
        let is_system_tx = transaction_data.is_system_tx();
        let is_genesis_tx = transaction_data.is_genesis_tx();
        let is_end_of_epoch_tx = transaction_data.is_end_of_epoch_tx();
        let is_sponsored_tx = transaction_data.is_sponsored_tx();

        println!("Authorizing transaction with the following details:");
        println!("Sender Address: {}", sender_address);
        println!("Kind: {:?}", kind);
        println!("Gas Owner: {}", gas_owner);
        // println!("Gas: {:#?}", gas);
        println!("Gas Price: {}", gas_price);
        println!("Gas Budget: {}", gas_budget);
        println!("Is System Tx: {}", is_system_tx);
        println!("Is Genesis Tx: {}", is_genesis_tx);
        println!("Is End Of Epoch Tx: {}", is_end_of_epoch_tx);
        println!("Is Sponsored Tx: {}", is_sponsored_tx);

        let client_list_view = self
            .client_list_view
            .load(CLIENT_LIST_VIEW_ID)
            .await?
            .ok_or_else(|| anyhow!("Client list view not found"))?;

        println!("Sender address: {}", sender_address);

        let client_view = client_list_view
            .into_inner()
            .into_values()
            .find(|client_view| client_view.wallet_address == sender_address)
            .expect("FIXME: handle error properly");

        if let Some(group_id) = client_view.group_id {
            let group_view = self
                .group_view
                .load(&group_id.to_string())
                .await?
                .ok_or_else(|| anyhow!("Group view not found for id `{}`", group_id))?;

            if group_view.balance < gas_budget {
                return Err(anyhow!(
                    "Insufficient balance in group `{}` for transaction. Available: {}, Required: {}",
                    group_id,
                    group_view.balance,
                    gas_budget
                ));
            }
        } else {
            unimplemented!("Handle clients without a group");
        }

        Ok(())
    }
}
