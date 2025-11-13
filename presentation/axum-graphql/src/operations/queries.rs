use anyhow::Result;
use application::{
    services::allocation_service::{get_iota_eur_price, get_iota_usd_price},
    views::{
        client::ClientView,
        client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
        group::GroupView,
        group_list::{GROUP_LIST_VIEW_ID, GroupListView},
        sponsor_wallet::{SPONSOR_WALLET_VIEW_ID, SponsorWalletView},
        sponsorship_transaction::SponsorshipTransactionView,
        sponsorship_transaction_list::{
            SPONSORSHIP_TRANSACTION_LIST_VIEW_ID, SponsorshipTransactionListView,
        },
    },
};
use async_graphql::{
    Object,
    connection::{
        Connection, DefaultConnectionName, DefaultEdgeName, Edge, EmptyFields, EnableNodesField,
        query,
    },
};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use chrono::{DateTime, Utc};
use cqrs_es::persist::ViewRepository;
use mongo_es::MongoViewRepository;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::operations::{
    ClientDto, GroupDto, Metrics, SponsorWalletDto, SponsorshipTransactionDto,
};

#[derive(Clone)]
pub struct QueryRoot {
    sponsor_wallet_view: Arc<MongoViewRepository<SponsorWalletView, SponsorWallet>>,
    client_view: Arc<MongoViewRepository<ClientView, Client>>,
    client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
    sponsorship_transaction_list_view:
        Arc<MongoViewRepository<SponsorshipTransactionListView, Group>>,
    group_view: Arc<MongoViewRepository<GroupView, Group>>,
    group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,
}

impl QueryRoot {
    pub fn new(
        sponsor_wallet_view: Arc<MongoViewRepository<SponsorWalletView, SponsorWallet>>,
        client_view: Arc<MongoViewRepository<ClientView, Client>>,
        client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
        sponsorship_transaction_list_view: Arc<
            MongoViewRepository<SponsorshipTransactionListView, Group>,
        >,
        group_view: Arc<MongoViewRepository<GroupView, Group>>,
        group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            sponsor_wallet_view,
            client_view,
            client_list_view,
            sponsorship_transaction_list_view,
            group_view,
            group_list_view,
        }
    }
}

#[Object]
impl QueryRoot {
    /// Returns the sponsor wallet
    #[instrument(skip(self))]
    async fn get_sponsor_wallet(&self) -> Result<Option<SponsorWalletDto>> {
        let mut sponsor_wallet = self
            .sponsor_wallet_view
            .load(SPONSOR_WALLET_VIEW_ID)
            .await?
            .map(SponsorWalletDto::from);

        if let Some(sponsor_wallet) = &mut sponsor_wallet {
            let transactions = self
                .sponsorship_transaction_list_view
                .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                .await?
                .map(|sponsorship_transaction_list_view| {
                    sponsorship_transaction_list_view
                        .into_inner()
                        .values()
                        .cloned()
                        .collect::<Vec<SponsorshipTransactionView>>()
                })
                .unwrap_or_default();

            sponsor_wallet.metrics = Metrics::from(transactions);

            // --- Calculate new estimation fields ---
            if sponsor_wallet.metrics.all_time.average_transaction_fee > 0.0 {
                sponsor_wallet.estimated_remaining_transactions = Some(
                    (sponsor_wallet.balance as f64
                        / sponsor_wallet.metrics.all_time.average_transaction_fee)
                        as u64,
                );
            }
            if sponsor_wallet
                .metrics
                .all_time
                .average_daily_sponsored_amount
                > 0.0
            {
                let days_remaining = sponsor_wallet.balance as f64
                    / sponsor_wallet
                        .metrics
                        .all_time
                        .average_daily_sponsored_amount;
                sponsor_wallet.estimated_depletion_date = Some(
                    Utc::now()
                        + chrono::Duration::milliseconds(
                            (days_remaining * 24.0 * 3600.0 * 1000.0) as i64,
                        ),
                );
            }
        }

        Ok(sponsor_wallet)
    }

    /// Returns a client by ID
    #[instrument(skip(self))]
    async fn get_client(&self, client_id: Uuid) -> Result<Option<ClientDto>> {
        let mut client = self
            .client_view
            .load(&client_id.to_string())
            .await?
            .and_then(|client_view| {
                (!client_view.is_deleted).then(|| ClientDto::from(client_view))
            });

        if let Some(client) = &mut client {
            let transactions = self
                .sponsorship_transaction_list_view
                .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                .await?
                .map(|sponsorship_transaction_list_view| {
                    sponsorship_transaction_list_view
                        .into_inner()
                        .values()
                        .cloned()
                        .filter(|tx| tx.client_id == client_id)
                        .collect::<Vec<SponsorshipTransactionView>>()
                })
                .unwrap_or_default();

            client.metrics = Metrics::from(transactions);
        }

        Ok(client)
    }

    /// Returns the list of all clients
    #[instrument(skip(self))]
    async fn get_client_list(&self) -> Result<Vec<ClientDto>> {
        Ok(self
            .client_list_view
            .load(CLIENT_LIST_VIEW_ID)
            .await?
            .map(|client_list_view| {
                client_list_view
                    .into_inner()
                    .values()
                    .cloned()
                    .filter_map(|client_view| {
                        (!client_view.is_deleted).then(|| ClientDto::from(client_view))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Returns the list of all groups
    #[instrument(skip(self), fields(after = %after, before = %before))]
    async fn get_transaction_list(
        &self,
        after: DateTime<Utc>,
        before: DateTime<Utc>,
    ) -> Result<Vec<SponsorshipTransactionDto>> {
        let mut sponsorship_transaction_list: Vec<_> = self
            .sponsorship_transaction_list_view
            .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
            .await?
            .map(|sponsorship_transaction_list_view| {
                sponsorship_transaction_list_view
                    .into_inner()
                    .values()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        sponsorship_transaction_list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let connection = query(
            Some(after.to_rfc3339()),
            Some(before.to_rfc3339()),
            None,
            None,
            |after: Option<String>, before, _first, _last| async move {
                let start = DateTime::parse_from_rfc3339(&after.unwrap_or_default()).unwrap();
                let end = DateTime::parse_from_rfc3339(
                    &before.unwrap_or_else(|| Utc::now().to_rfc3339()),
                )
                .unwrap();

                let mut connection: Connection<
                    String,
                    SponsorshipTransactionDto,
                    EmptyFields,
                    EmptyFields,
                    DefaultConnectionName,
                    DefaultEdgeName,
                    EnableNodesField,
                > = Connection::new(true, true);
                for sponsorship_transaction in
                    sponsorship_transaction_list
                        .iter()
                        .filter(|sponsorship_transaction| {
                            sponsorship_transaction.timestamp >= start
                                && sponsorship_transaction.timestamp <= end
                        })
                {
                    let edge = Edge::new(
                        sponsorship_transaction.group_id.to_string(),
                        SponsorshipTransactionDto::from(sponsorship_transaction.clone()),
                    );
                    connection.edges.push(edge);
                }

                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
        .unwrap();

        Ok(connection
            .edges
            .into_iter()
            .map(|edge: Edge<_, _, _, _>| edge.node)
            .collect())
    }

    /// Returns a group by ID
    #[instrument(skip(self))]
    async fn get_group(&self, group_id: Uuid) -> Result<Option<GroupDto>> {
        let mut group = self
            .group_view
            .load(&group_id.to_string())
            .await?
            .and_then(|group_view| (!group_view.is_deleted).then(|| GroupDto::from(group_view)));

        if let Some(group) = &mut group {
            let transactions = self
                .sponsorship_transaction_list_view
                .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                .await?
                .map(|sponsorship_transaction_list_view| {
                    sponsorship_transaction_list_view
                        .into_inner()
                        .values()
                        .cloned()
                        .filter(|tx| tx.group_id == group_id)
                        .collect::<Vec<SponsorshipTransactionView>>()
                })
                .unwrap_or_default();

            group.metrics = Metrics::from(transactions);

            // --- Calculate new estimation fields ---
            if group.metrics.all_time.average_transaction_fee > 0.0 {
                group.estimated_remaining_transactions = Some(
                    (group.balance as f64 / group.metrics.all_time.average_transaction_fee) as u64,
                );
            }
            if group.metrics.all_time.average_daily_sponsored_amount > 0.0 {
                let days_remaining =
                    group.balance as f64 / group.metrics.all_time.average_daily_sponsored_amount;
                group.estimated_depletion_date = Some(
                    Utc::now()
                        + chrono::Duration::milliseconds(
                            (days_remaining * 24.0 * 3600.0 * 1000.0) as i64,
                        ),
                );
            }
        }

        Ok(group)
    }

    /// Returns the list of all groups
    #[instrument(skip(self))]
    async fn get_group_list(&self) -> Result<Vec<GroupDto>> {
        Ok(self
            .group_list_view
            .load(GROUP_LIST_VIEW_ID)
            .await?
            .map(|group_list_view| {
                group_list_view
                    .into_inner()
                    .values()
                    .cloned()
                    .filter_map(|group_view| {
                        (!group_view.is_deleted).then(|| GroupDto::from(group_view))
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Returns the current IOTA to EUR price
    async fn get_iota_eur_price(&self) -> Result<f64> {
        Ok(get_iota_eur_price().await)
    }

    /// Returns the current IOTA to USD price
    async fn get_iota_usd_price(&self) -> Result<f64> {
        Ok(get_iota_usd_price().await)
    }
}
