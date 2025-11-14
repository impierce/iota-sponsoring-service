use std::sync::Arc;

use anyhow::{Result, anyhow};
use balance_management::{
    client::aggregate::Client,
    group::{aggregate::Group, command::GroupCommand},
};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use iota_types::transaction;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use wallet_integration::sponsor_wallet::{aggregate::SponsorWallet, command::SponsorWalletCommand};

use crate::views::{
    client::ClientView,
    client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
    group::GroupView,
    group_list::{GROUP_LIST_VIEW_ID, GroupListView},
    sponsor_wallet::SponsorWalletView,
};
pub struct AllocationService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet>,
    CES: EventStore<Client>,
    GES: EventStore<Group>,
{
    sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
    _client_handler: Arc<CqrsFramework<Client, CES>>,
    group_handler: Arc<CqrsFramework<Group, GES>>,
    sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
    _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
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
        _client_handler: Arc<CqrsFramework<Client, CES>>,
        group_handler: Arc<CqrsFramework<Group, GES>>,
        sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
        _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
        client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
        group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            sponsor_wallet_handler,
            _client_handler,
            group_handler,
            sponsor_wallet_view,
            _client_view,
            client_list_view,
            group_view,
            group_list_view,
        }
    }

    #[instrument(skip(self), fields(sponsor_wallet_id = %sponsor_wallet_id, name = %name))]
    pub async fn update_sponsor_wallet_name(
        &self,
        sponsor_wallet_id: String,
        name: String,
    ) -> Result<SponsorWalletView> {
        info!("Updating sponsor wallet name");
        let command = SponsorWalletCommand::UpdateSponsorWalletName { name };

        debug!("Dispatching UpdateSponsorWalletName command");
        self.sponsor_wallet_handler
            .execute(&sponsor_wallet_id, command)
            .await?;

        let view = self
            .sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                let err_msg = format!(
                    "Sponsor wallet view not found after updating name for `{sponsor_wallet_id}`"
                );
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

        debug!(?view, "Successfully loaded updated sponsor wallet view");
        info!("Successfully updated sponsor wallet name");
        Ok(view)
    }

    #[instrument(skip(self), fields(sponsor_wallet_id = %sponsor_wallet_id, logo_uri = ?logo_uri))]
    pub async fn update_sponsor_wallet_logo_uri(
        &self,
        sponsor_wallet_id: String,
        logo_uri: Option<url::Url>,
    ) -> Result<SponsorWalletView> {
        info!("Updating sponsor wallet logo URI");
        let command = SponsorWalletCommand::UpdateSponsorWalletLogoUri { logo_uri };

        debug!("Dispatching UpdateSponsorWalletLogoUri command");
        self.sponsor_wallet_handler
            .execute(&sponsor_wallet_id, command)
            .await?;

        let view = self
            .sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                let err_msg = format!("Sponsor wallet view not found after updating logo URI for `{sponsor_wallet_id}`");
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

        debug!(?view, "Successfully loaded updated sponsor wallet view");
        info!("Successfully updated sponsor wallet logo URI");
        Ok(view)
    }

    pub async fn get_sponsor_wallet_view(
        &self,
        sponsor_wallet_id: String,
    ) -> Result<SponsorWalletView> {
        info!("Getting sponsor wallet view");
        let view = self
            .sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                let err_msg = format!("Sponsor wallet view not found for id `{sponsor_wallet_id}`");
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

        debug!(?view, "Successfully loaded sponsor wallet view");
        info!("Successfully got sponsor wallet view");
        Ok(view)
    }

    #[instrument(skip(self), fields(sponsor_wallet_id = %sponsor_wallet_id, new_balance = %new_balance))]
    pub async fn record_balance_update(
        &self,
        sponsor_wallet_id: String,
        new_balance: u64,
    ) -> Result<SponsorWalletView> {
        info!("Recording sponsor wallet balance update");
        let command = SponsorWalletCommand::RecordBalanceUpdate { new_balance };

        debug!("Dispatching RecordBalanceUpdate command");
        self.sponsor_wallet_handler
            .execute(&sponsor_wallet_id, command)
            .await?;

        let view = self
            .sponsor_wallet_view
            .load(&sponsor_wallet_id)
            .await?
            .ok_or_else(|| {
                let err_msg = format!("Sponsor wallet view not found after recording balance update for `{sponsor_wallet_id}`");
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

        debug!(?view, "Successfully loaded updated sponsor wallet view");
        info!("Successfully recorded sponsor wallet balance update");
        Ok(view)
    }

    #[instrument(skip(self), fields(sender_address = %sender_address, transaction_fee = %transaction_fee))]
    pub async fn record_transaction_fee_paid(
        &self,
        sender_address: String,
        transaction_fee: u64,
    ) -> Result<()> {
        info!("Recording transaction fee paid by client");
        debug!("Loading client list view to find client by address");
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

        debug!(client_id = %client_view.client_id, "Found client for sender address");

        let transaction_fee_iot = transaction_fee as f64 / 1_000_000_000.0;

        let transaction_fee_eur =
            get_iota_eur_price().await * (transaction_fee as f64 / 1_000_000_000.0);

        let transaction_fee_usd =
            get_iota_usd_price().await * (transaction_fee as f64 / 1_000_000_000.0);

        if let Some(group_id) = &client_view.group_id {
            debug!(group_id = %group_id, "Client belongs to a group, dispatching `RecordTransactionFeePaid` command");
            let command = GroupCommand::RecordTransactionFeePaid {
                client_id: client_view.client_id,
                client_name: client_view.name.clone(),
                transaction_fee,
                transaction_fee_iot,
                transaction_fee_eur,
                transaction_fee_usd,
            };

            self.group_handler
                .execute(&group_id.to_string(), command)
                .await
                .map_err(|e| anyhow!("Failed to record transaction fee paid to group: {}", e))?;
            info!("Successfully recorded transaction fee paid to group");
        } else {
            warn!("Client does not belong to any group. Skipping group fee recording.");
        }

        Ok(())
    }

    #[instrument(skip(self), fields(sponsor_wallet_id = %sponsor_wallet_id, group_id = %group_id, amount = %amount))]
    pub async fn allocate_funds_to_group(
        &self,
        sponsor_wallet_id: String,
        group_id: Uuid,
        amount: u64,
    ) -> Result<GroupView> {
        info!("Allocating funds to group");
        debug!("Loading sponsor wallet and group list views to check balances");
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
                    "Group list view not found for id `{}` during allocation",
                    GROUP_LIST_VIEW_ID
                )
            })?;

        let allocated_balance = group_list_view
            .into_inner()
            .into_values()
            .map(|group_view| group_view.balance)
            .sum::<u64>();

        debug!(
            sponsor_balance = sponsor_wallet_view.balance,
            already_allocated = allocated_balance,
            "Checking for sufficient funds"
        );
        if sponsor_wallet_view.balance < allocated_balance + amount {
            let err_msg = format!(
                "Insufficient balance in sponsor wallet `{}` for allocation. Available: {}, Requested: {}",
                sponsor_wallet_id, sponsor_wallet_view.balance, amount
            );
            error!("{}", err_msg);
            return Err(anyhow!(err_msg));
        }

        let command = GroupCommand::AllocateFundsToGroup { group_id, amount };

        debug!("Dispatching `AllocateFundsToGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!("Group view not found after allocating funds to group `{group_id}`")
            })?;

        info!("Successfully allocated funds to group");
        Ok(view)
    }

    #[instrument(skip(self), fields(group_id = %group_id, amount = %amount))]
    pub async fn withdraw_funds_from_group(
        &self,
        group_id: Uuid,
        amount: u64,
    ) -> Result<GroupView> {
        info!("Withdrawing funds from group");
        let command = GroupCommand::WithdrawFundsFromGroup { group_id, amount };

        debug!("Dispatching WithdrawFundsFromGroup command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let view = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .ok_or_else(|| {
                anyhow!("Group view not found after withdrawing funds from group `{group_id}`")
            })?;

        info!("Successfully withdrew funds from group");
        Ok(view)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BitvavoIotaEurPriceTickerResponse {
    pub market: String,
    pub price: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceIotaUsdtPriceTickerResponse {
    pub symbol: String,
    pub price: String,
}

// TODO: Remove hardcoded fallback prices and implement proper error handling!
pub async fn get_iota_eur_price() -> f64 {
    if let Ok(response) =
        reqwest::get("https://api.bitvavo.com/v2/ticker/price?market=MIOTA-EUR").await
    {
        if let Ok(response) = response.json::<BitvavoIotaEurPriceTickerResponse>().await {
            debug!(?response, "Fetched IOTA-EUR price from Bitvavo");

            return response.price.parse::<f64>().unwrap_or(0.13);
        } else {
            warn!("Failed to parse Bitvavo IOTA-EUR price ticker response");
        }
    } else {
        warn!("Failed to fetch IOTA-EUR price from Bitvavo");
    }

    0.13
}

// TODO: Remove hardcoded fallback prices and implement proper error handling!
pub async fn get_eur_iota_price() -> f64 {
    let iota_eur_price = get_iota_eur_price().await;
    if iota_eur_price > 0.0 {
        1.0 / iota_eur_price
    } else {
        7.69 // Fallback price
    }
}

// TODO: Remove hardcoded fallback prices and implement proper error handling!
pub async fn get_iota_usd_price() -> f64 {
    if let Ok(response) =
        reqwest::get("https://api.binance.com/api/v3/ticker/price?symbol=IOTAUSDT").await
    {
        if let Ok(response) = response.json::<BinanceIotaUsdtPriceTickerResponse>().await {
            debug!(?response, "Fetched IOTA-USDT price from Binance");

            return response.price.parse::<f64>().unwrap_or(0.15);
        } else {
            warn!("Failed to parse Binance IOTA-USDT price ticker response");
        }
    } else {
        warn!("Failed to fetch IOTA-USDT price from Binance");
    }

    0.15
}

// TODO: Remove hardcoded fallback prices and implement proper error handling!
pub async fn get_usd_iota_price() -> f64 {
    let iota_usd_price = get_iota_usd_price().await;
    if iota_usd_price > 0.0 {
        1.0 / iota_usd_price
    } else {
        6.67 // Fallback price
    }
}

pub async fn get_conversion_rates() -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
    let iot_to_eur = get_iota_eur_price().await;
    let eur_to_iot = get_eur_iota_price().await;
    let iot_to_usd = get_iota_usd_price().await;
    let usd_to_iot = get_usd_iota_price().await;

    let nano_to_iot = 1.0 / 1_000_000_000.0;
    let nano_to_eur = nano_to_iot * iot_to_eur;
    let nano_to_usd = nano_to_iot * iot_to_usd;

    let iot_to_nano = 1_000_000_000.0;
    let eur_to_nano = eur_to_iot * iot_to_nano;
    let usd_to_nano = usd_to_iot * iot_to_nano;

    (
        nano_to_iot,
        iot_to_nano,
        nano_to_eur,
        eur_to_nano,
        nano_to_usd,
        usd_to_nano,
        iot_to_eur,
        eur_to_iot,
        iot_to_usd,
        usd_to_iot,
    )
}
