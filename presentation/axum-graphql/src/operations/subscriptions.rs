use super::{ClientDto, GroupDto};
use crate::operations::{ConversionRatesDto, Metrics, SponsorWalletDto};
use application::{
    services::allocation_service::get_conversion_rates,
    views::{
        client::ClientView,
        group::GroupView,
        sponsor_wallet::SponsorWalletView,
        sponsorship_transaction::SponsorshipTransactionView,
        sponsorship_transaction_list::{
            SPONSORSHIP_TRANSACTION_LIST_VIEW_ID, SponsorshipTransactionListView,
        },
    },
};
use async_graphql::Subscription;
use balance_management::group::aggregate::Group;
use chrono::Utc;
use cqrs_es::persist::ViewRepository as _;
use futures_util::{Stream, StreamExt};
use mongo_es::MongoViewRepository;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::instrument;

#[derive(Clone)]
pub struct SubscriptionRoot {
    sponsor_wallet_query_receiver: Arc<broadcast::Receiver<SponsorWalletView>>,
    client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
    group_query_receiver: Arc<broadcast::Receiver<GroupView>>,

    sponsorship_transaction_list_view:
        Arc<MongoViewRepository<SponsorshipTransactionListView, Group>>,
}

impl SubscriptionRoot {
    pub fn new(
        sponsor_wallet_query_receiver: Arc<broadcast::Receiver<SponsorWalletView>>,
        client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
        group_query_receiver: Arc<broadcast::Receiver<GroupView>>,

        sponsorship_transaction_list_view: Arc<
            MongoViewRepository<SponsorshipTransactionListView, Group>,
        >,
    ) -> Self {
        Self {
            sponsor_wallet_query_receiver,
            client_query_receiver,
            group_query_receiver,

            sponsorship_transaction_list_view,
        }
    }
}

// TODO: fix the duplicated code from `queries.rs`
#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to sponsor wallet updates
    #[instrument(skip(self))]
    async fn sponsor_wallet_updates(&self) -> impl Stream<Item = SponsorWalletDto> {
        let receiver = self.sponsor_wallet_query_receiver.resubscribe();
        let transaction_list_view = self.sponsorship_transaction_list_view.clone();

        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .then(move |result| {
                let transaction_list_view = transaction_list_view.clone();
                async move {
                    let mut sponsor_wallet: Option<SponsorWalletDto> =
                        result.map(|view| view.into()).ok();

                    if let Some(sponsor_wallet) = &mut sponsor_wallet {
                        let transactions = transaction_list_view
                            .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                            .await
                            .ok()
                            .flatten()
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

                    sponsor_wallet
                }
            })
            .filter_map(|x| async move { x })
    }

    /// Subscribe to client updates
    #[instrument(skip(self))]
    async fn client_updates(&self) -> impl Stream<Item = ClientDto> {
        let receiver = self.client_query_receiver.resubscribe();
        let transaction_list_view = self.sponsorship_transaction_list_view.clone();

        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .then(move |result| {
                let transaction_list_view = transaction_list_view.clone();
                async move {
                    let mut client: Option<ClientDto> = result.map(|view| view.into()).ok();

                    if let Some(client) = &mut client {
                        let transactions = transaction_list_view
                            .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                            .await
                            .ok()
                            .flatten()
                            .map(|sponsorship_transaction_list_view| {
                                sponsorship_transaction_list_view
                                    .into_inner()
                                    .values()
                                    .cloned()
                                    .collect::<Vec<SponsorshipTransactionView>>()
                            })
                            .unwrap_or_default();

                        client.metrics = Metrics::from(transactions);
                    }

                    client
                }
            })
            .filter_map(|x| async move { x })
    }

    // Subscribe to group updates
    #[instrument(skip(self))]
    async fn group_updates(&self) -> impl Stream<Item = GroupDto> {
        let receiver = self.group_query_receiver.resubscribe();
        let transaction_list_view = self.sponsorship_transaction_list_view.clone();

        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .then(move |result| {
                let transaction_list_view = transaction_list_view.clone();
                async move {
                    let mut group: Option<GroupDto> = result.map(|view| view.into()).ok();

                    if let Some(group) = &mut group {
                        let transactions = transaction_list_view
                            .load(SPONSORSHIP_TRANSACTION_LIST_VIEW_ID)
                            .await
                            .ok()
                            .flatten()
                            .map(|sponsorship_transaction_list_view| {
                                sponsorship_transaction_list_view
                                    .into_inner()
                                    .values()
                                    .cloned()
                                    .collect::<Vec<SponsorshipTransactionView>>()
                            })
                            .unwrap_or_default();

                        group.metrics = Metrics::from(transactions);

                        // --- Calculate new estimation fields ---
                        if group.metrics.all_time.average_transaction_fee > 0.0 {
                            group.estimated_remaining_transactions = Some(
                                (group.balance as f64
                                    / group.metrics.all_time.average_transaction_fee)
                                    as u64,
                            );
                        }
                        if group.metrics.all_time.average_daily_sponsored_amount > 0.0 {
                            let days_remaining = group.balance as f64
                                / group.metrics.all_time.average_daily_sponsored_amount;
                            group.estimated_depletion_date = Some(
                                Utc::now()
                                    + chrono::Duration::milliseconds(
                                        (days_remaining * 24.0 * 3600.0 * 1000.0) as i64,
                                    ),
                            );
                        }
                    }

                    group
                }
            })
            .filter_map(|x| async move { x })
    }

    /// Subscribe to conversion rate updates
    #[instrument(skip(self))]
    async fn conversion_rate_updates(&self) -> impl Stream<Item = ConversionRatesDto> {
        let interval = tokio::time::interval(std::time::Duration::from_secs(120));

        tokio_stream::wrappers::IntervalStream::new(interval)
            .then(|_| async { ConversionRatesDto::from(get_conversion_rates().await) })
    }
}
