use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use base64::prelude::*;
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use iota_gas_station::access_controller::hook::ExecuteTxHookRequest;
use iota_types::transaction::{TransactionData, TransactionDataAPI};
use tracing::{debug, error, info, instrument, warn};
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::views::{
    client::ClientView,
    client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
    group::GroupView,
    group_list::GroupListView,
    sponsor_wallet::SponsorWalletView,
};
pub struct AuthorizeTransactionService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet>,
    CES: EventStore<Client>,
    GES: EventStore<Group>,
{
    _sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
    _client_handler: Arc<CqrsFramework<Client, CES>>,
    _group_handler: Arc<CqrsFramework<Group, GES>>,
    _sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
    _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
    client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
    group_view: Arc<dyn ViewRepository<GroupView, Group>>,
    _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
}

impl<SWES, CES, GES> AuthorizeTransactionService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet> + 'static,
    CES: EventStore<Client> + 'static,
    GES: EventStore<Group> + 'static,
{
    pub fn new(
        _sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
        _client_handler: Arc<CqrsFramework<Client, CES>>,
        _group_handler: Arc<CqrsFramework<Group, GES>>,
        _sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
        _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
        client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
        group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            _sponsor_wallet_handler,
            _client_handler,
            _group_handler,
            _sponsor_wallet_view,
            _client_view,
            client_list_view,
            group_view,
            _group_list_view,
        }
    }

    #[instrument(skip(self, transaction_data), fields(reservation_id = %transaction_data.execute_tx_request.payload.reservation_id))]
    pub async fn authorize_transaction(
        &self,
        transaction_data: ExecuteTxHookRequest,
    ) -> Result<()> {
        info!("Authorizing transaction");

        let tx_data: TransactionData = BASE64_STANDARD
            .decode(&transaction_data.execute_tx_request.payload.tx_bytes)
            .context("Failed to decode base64 transaction data")
            .and_then(|bytes| {
                bcs::from_bytes(&bytes).context("Failed to parse BCS bytes to `TransactionData`")
            })?;

        let sender_address = tx_data.sender().to_string();
        let gas_budget = tx_data.gas_budget();

        debug!(
            sender_address = %sender_address,
            gas_owner = %tx_data.gas_owner(),
            gas_budget = %gas_budget,
            gas_price = %tx_data.gas_price(),
            is_sponsored = %tx_data.is_sponsored_tx(),
            "Parsed transaction details"
        );

        debug!("Loading client list view to find client by sender address");
        let client_list_view = self
            .client_list_view
            .load(CLIENT_LIST_VIEW_ID)
            .await?
            .ok_or_else(|| anyhow!("Client list view not found"))?;

        let client_view = client_list_view
            .into_inner()
            .into_values()
            .find(|client_view| client_view.wallet_address == sender_address)
            .ok_or_else(|| anyhow!("Client not found for sender address `{}`", sender_address))?;

        debug!(client_id = %client_view.client_id, "Found client for sender address");

        if let Some(group_id) = client_view.group_id {
            debug!(group_id = %group_id, "Client belongs to a group, checking group balance");
            let group_view = self
                .group_view
                .load(&group_id.to_string())
                .await?
                .ok_or_else(|| anyhow!("Group view not found for id `{}`", group_id))?;

            if group_view.balance >= gas_budget {
                info!(
                    group_balance = group_view.balance,
                    "Sufficient balance. Authorizing transaction."
                );
                Ok(())
            } else {
                let err_msg = format!(
                    "Insufficient balance in group `{}` for transaction. Available: {}, Required: {}",
                    group_id, group_view.balance, gas_budget
                );
                error!("{}", err_msg);
                Err(anyhow!(err_msg))
            }
        } else {
            warn!(client_id = %client_view.client_id, "Transaction denied: Client does not belong to a group");
            Err(anyhow!(
                "Client `{}` is not assigned to a group",
                client_view.client_id
            ))
        }
    }
}
