pub mod mutations;
pub mod queries;
pub mod subscriptions;

use application::views::client::ClientView;
use application::views::group::GroupView;
use application::views::sponsor_wallet::SponsorWalletView;
use application::views::sponsorship_transaction::SponsorshipTransactionView;
use async_graphql::{InputObject, SimpleObject};
use balance_management::group::aggregate::{Status, Variant};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::debug;
use url::Url;
use uuid::Uuid;

pub const NANO_PER_IOTA: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, SimpleObject, Default)]
pub struct Period {
    pub total_transactions: u64,

    pub total_sponsored_amount: u64,
    pub total_sponsored_amount_iot: f64,
    pub total_sponsored_amount_eur: f64,
    pub total_sponsored_amount_usd: f64,

    pub average_transaction_fee: f64,
    pub average_transaction_fee_iot: f64,
    pub average_transaction_fee_eur: f64,
    pub average_transaction_fee_usd: f64,

    pub average_daily_transactions: f64,

    pub average_daily_sponsored_amount: f64,
    pub average_daily_sponsored_amount_iot: f64,
    pub average_daily_sponsored_amount_eur: f64,
    pub average_daily_sponsored_amount_usd: f64,

    pub max_transaction_fee: u64,
    pub max_transaction_fee_iot: f64,
    pub max_transaction_fee_eur: f64,
    pub max_transaction_fee_usd: f64,

    pub min_transaction_fee: u64,
    pub min_transaction_fee_iot: f64,
    pub min_transaction_fee_eur: f64,
    pub min_transaction_fee_usd: f64,
}

#[derive(Debug, Clone, SimpleObject, Default)]
pub struct Metrics {
    pub first_transaction: Option<DateTime<Utc>>,
    pub last_transaction: Option<DateTime<Utc>>,

    pub all_time: Period,
    pub last_7_days: Period,
    pub last_30_days: Period,
    pub last_90_days: Period,
}

impl From<Vec<SponsorshipTransactionView>> for Metrics {
    fn from(transactions: Vec<SponsorshipTransactionView>) -> Self {
        if transactions.is_empty() {
            return Metrics::default();
        }

        let mut metrics = Metrics::default();
        metrics.first_transaction = transactions.iter().map(|tx| tx.timestamp).min();
        metrics.last_transaction = transactions.iter().map(|tx| tx.timestamp).max();

        // --- Step 1: Define time windows ONCE before the loop ---
        let now = Utc::now();
        let seven_days_ago = now - chrono::Duration::days(7);
        let thirty_days_ago = now - chrono::Duration::days(30);
        let ninety_days_ago = now - chrono::Duration::days(90);

        debug!(
            "Calculating metrics from {} transactions",
            transactions.len()
        );
        debug!(
            "Time windows - 7 days ago: {}, 30 days ago: {}, 90 days ago: {}",
            seven_days_ago, thirty_days_ago, ninety_days_ago
        );

        // --- Step 2: Accumulate totals in a single pass ---
        for tx in &transactions {
            // Always update the all_time period
            update_period_totals(&mut metrics.all_time, tx);

            // Conditionally update the other periods
            if tx.timestamp >= seven_days_ago {
                update_period_totals(&mut metrics.last_7_days, tx);
            }
            if tx.timestamp >= thirty_days_ago {
                update_period_totals(&mut metrics.last_30_days, tx);
            }
            if tx.timestamp >= ninety_days_ago {
                update_period_totals(&mut metrics.last_90_days, tx);
            }
        }

        // --- Step 3: Calculate final averages ONCE after the loop ---
        finalize_period_calculations(&mut metrics.all_time, 365 * 100); // Use a large number for all_time days
        finalize_period_calculations(&mut metrics.last_7_days, 7);
        finalize_period_calculations(&mut metrics.last_30_days, 30);
        finalize_period_calculations(&mut metrics.last_90_days, 90);

        // Correctly calculate the daily average for all_time based on actual transaction span
        if let (Some(first), Some(last)) = (metrics.first_transaction, metrics.last_transaction) {
            let days_span = (last.date_naive() - first.date_naive()).num_days().max(1) as f64;
            if metrics.all_time.total_transactions > 0 {
                metrics.all_time.average_daily_transactions =
                    metrics.all_time.total_transactions as f64 / days_span;
                metrics.all_time.average_daily_sponsored_amount =
                    metrics.all_time.total_sponsored_amount as f64 / days_span;
                metrics.all_time.average_daily_sponsored_amount_eur =
                    metrics.all_time.total_sponsored_amount_eur / days_span;
                metrics.all_time.average_daily_sponsored_amount_usd =
                    metrics.all_time.total_sponsored_amount_usd / days_span;
            }
        }

        debug!("Final calculated metrics: {:#?}", metrics);

        metrics
    }
}

/// Helper function to accumulate totals for a given period.
fn update_period_totals(period: &mut Period, tx: &SponsorshipTransactionView) {
    period.total_transactions += 1;
    period.total_sponsored_amount += tx.transaction_fee;
    period.total_sponsored_amount_iot += tx.transaction_fee as f64 / NANO_PER_IOTA;
    period.total_sponsored_amount_eur += tx.transaction_fee_eur;
    period.total_sponsored_amount_usd += tx.transaction_fee_usd;

    // Update max
    if tx.transaction_fee > period.max_transaction_fee {
        period.max_transaction_fee = tx.transaction_fee;
        period.max_transaction_fee_iot = tx.transaction_fee as f64 / NANO_PER_IOTA;
        period.max_transaction_fee_eur = tx.transaction_fee_eur;
        period.max_transaction_fee_usd = tx.transaction_fee_usd;
    }

    // Update min (initialize with first transaction's fee)
    if period.min_transaction_fee == 0 || tx.transaction_fee < period.min_transaction_fee {
        period.min_transaction_fee = tx.transaction_fee;
        period.min_transaction_fee_iot = tx.transaction_fee as f64 / NANO_PER_IOTA;
        period.min_transaction_fee_eur = tx.transaction_fee_eur;
        period.min_transaction_fee_usd = tx.transaction_fee_usd;
    }
}

/// Helper function to calculate averages after totals are accumulated.
fn finalize_period_calculations(period: &mut Period, days_in_period: i64) {
    if period.total_transactions == 0 {
        return;
    }

    let total_tx_f64 = period.total_transactions as f64;
    let days_f64 = days_in_period as f64;

    // Overall Averages
    period.average_transaction_fee = period.total_sponsored_amount as f64 / total_tx_f64;
    period.average_transaction_fee_iot = period.total_sponsored_amount_iot / total_tx_f64;
    period.average_transaction_fee_eur = period.total_sponsored_amount_eur / total_tx_f64;
    period.average_transaction_fee_usd = period.total_sponsored_amount_usd / total_tx_f64;

    // Daily Averages
    period.average_daily_transactions = total_tx_f64 / days_f64;
    period.average_daily_sponsored_amount = period.total_sponsored_amount as f64 / days_f64;
    period.average_daily_sponsored_amount_iot = period.total_sponsored_amount_iot / days_f64;
    period.average_daily_sponsored_amount_eur = period.total_sponsored_amount_eur / days_f64;
    period.average_daily_sponsored_amount_usd = period.total_sponsored_amount_usd / days_f64;
}

#[derive(Debug, Clone, SimpleObject)]
pub struct SponsorWalletDto {
    pub sponsor_wallet_id: String,
    pub name: Option<String>,
    pub logo_uri: Option<Url>,
    pub address: String,
    pub balance: u64,

    pub metrics: Metrics,

    /// Estimated number of transactions remaining based on the current balance and all-time average fee.
    pub estimated_remaining_transactions: Option<u64>,
    /// Estimated date the balance will be depleted based on the all-time average daily sponsored amount.
    pub estimated_depletion_date: Option<DateTime<Utc>>,
}

impl From<SponsorWalletView> for SponsorWalletDto {
    fn from(view: SponsorWalletView) -> Self {
        let inner = view.into_inner();
        Self {
            sponsor_wallet_id: inner.sponsor_wallet_id,
            address: inner.address,
            balance: inner.balance,
            name: inner.name,
            logo_uri: inner.logo_uri,

            metrics: Metrics::default(),

            estimated_remaining_transactions: None,
            estimated_depletion_date: None,
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct ClientDto {
    pub client_id: Uuid,
    pub name: String,
    pub logo_uri: Option<Url>,
    pub website_uri: Option<Url>,
    pub wallet_address: String,
    pub balance: Option<u64>,
    pub group_id: Option<Uuid>,

    pub metrics: Metrics,
}

impl From<ClientView> for ClientDto {
    fn from(view: ClientView) -> Self {
        let inner = view.into_inner();
        Self {
            client_id: inner.client_id,
            name: inner.name,
            logo_uri: inner.logo_uri,
            website_uri: inner.website_uri,
            wallet_address: inner.wallet_address,
            balance: inner.balance,
            group_id: inner.group_id,

            metrics: Metrics::default(),
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct SponsorshipTransactionDto {
    pub group_id: Uuid,
    pub client_id: Uuid,
    pub client_name: String,
    pub transaction_fee: u64,
    pub transaction_fee_iot: f64,
    pub transaction_fee_eur: f64,
    pub transaction_fee_usd: f64,
    pub timestamp: DateTime<Utc>,
}

impl From<SponsorshipTransactionView> for SponsorshipTransactionDto {
    fn from(view: SponsorshipTransactionView) -> Self {
        Self {
            group_id: view.group_id,
            client_id: view.client_id,
            client_name: view.client_name,
            transaction_fee: view.transaction_fee,
            transaction_fee_iot: view.transaction_fee_iot,
            transaction_fee_eur: view.transaction_fee_eur,
            transaction_fee_usd: view.transaction_fee_usd,
            timestamp: view.timestamp,
        }
    }
}

#[derive(SimpleObject, InputObject, Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
pub struct StatusDto {
    pub variant: String,
}

impl From<Status> for StatusDto {
    fn from(status: Status) -> Self {
        let variant = match status.variant {
            Variant::Action => "action".to_string(),
            Variant::Warning => "warning".to_string(),
            Variant::Success => "success".to_string(),
        };

        Self { variant }
    }
}

impl Into<Status> for StatusDto {
    fn into(self) -> Status {
        let variant = match self.variant.as_str() {
            "action" => Variant::Action,
            "warning" => Variant::Warning,
            "success" => Variant::Success,
            _ => Variant::default(),
        };
        Status { variant }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct GroupDto {
    pub group_id: Uuid,
    pub name: String,
    pub logo_uri: Option<Url>,
    pub balance: u64,
    pub members: HashSet<Uuid>,
    pub status: StatusDto,

    pub metrics: Metrics,

    /// Estimated number of transactions remaining based on the current balance and all-time average fee.
    pub estimated_remaining_transactions: Option<u64>,
    /// Estimated date the balance will be depleted based on the all-time average daily sponsored amount.
    pub estimated_depletion_date: Option<DateTime<Utc>>,
}

impl From<GroupView> for GroupDto {
    fn from(view: GroupView) -> Self {
        let inner = view.into_inner();
        Self {
            group_id: inner.group_id,
            name: inner.name,
            logo_uri: inner.logo_uri,
            balance: inner.balance,
            members: inner.members,
            status: StatusDto::from(inner.status),

            metrics: Metrics::default(),

            estimated_remaining_transactions: None,
            estimated_depletion_date: None,
        }
    }
}

#[derive(Debug, Clone, SimpleObject)]
pub struct ConversionRatesDto {
    pub nano_to_iot: f64,
    pub iot_to_nano: f64,
    pub nano_to_eur: f64,
    pub eur_to_nano: f64,
    pub nano_to_usd: f64,
    pub usd_to_nano: f64,

    pub iot_to_eur: f64,
    pub eur_to_iot: f64,
    pub iot_to_usd: f64,
    pub usd_to_iot: f64,
}

impl From<(f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)> for ConversionRatesDto {
    fn from(rates: (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64)) -> Self {
        Self {
            nano_to_iot: rates.0,
            iot_to_nano: rates.1,
            nano_to_eur: rates.2,
            eur_to_nano: rates.3,
            nano_to_usd: rates.4,
            usd_to_nano: rates.5,

            iot_to_eur: rates.6,
            eur_to_iot: rates.7,
            iot_to_usd: rates.8,
            usd_to_iot: rates.9,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn create_tx(days_ago: i64, fee: u64, fee_eur: f64) -> SponsorshipTransactionView {
        SponsorshipTransactionView {
            sponsorship_transaction_id: Uuid::new_v4(),
            group_id: Uuid::new_v4(),
            client_id: Uuid::new_v4(),
            client_name: "Test Client".to_string(),
            transaction_fee: fee,
            transaction_fee_iot: fee as f64 / NANO_PER_IOTA,
            transaction_fee_eur: fee_eur,
            transaction_fee_usd: fee_eur * 1.1, // Mock USD conversion
            timestamp: Utc::now() - Duration::days(days_ago),
        }
    }

    #[test]
    fn test_metrics_from_empty_transactions() {
        let transactions = vec![];
        let metrics = Metrics::from(transactions);

        assert!(metrics.first_transaction.is_none());
        assert!(metrics.last_transaction.is_none());
        assert_eq!(metrics.all_time.total_transactions, 0);
        assert_eq!(metrics.last_7_days.total_transactions, 0);
    }

    #[test]
    fn test_metrics_from_single_transaction() {
        let transactions = vec![create_tx(5, 1000, 1.5)];
        let metrics = Metrics::from(transactions);

        assert!(metrics.first_transaction.is_some());
        assert!(metrics.last_transaction.is_some());
        assert_eq!(metrics.first_transaction, metrics.last_transaction);

        // Check all_time period
        assert_eq!(metrics.all_time.total_transactions, 1);
        assert_eq!(metrics.all_time.total_sponsored_amount, 1000);
        assert_eq!(metrics.all_time.min_transaction_fee, 1000);
        assert_eq!(metrics.all_time.max_transaction_fee, 1000);
        assert_eq!(metrics.all_time.average_transaction_fee, 1000.0);

        // Check last_7_days period (since it's 5 days ago)
        assert_eq!(metrics.last_7_days.total_transactions, 1);
        assert_eq!(metrics.last_7_days.total_sponsored_amount, 1000);
    }

    #[test]
    fn test_metrics_from_multiple_transactions() {
        let transactions = vec![
            create_tx(100, 1000, 1.0), // Oldest, min fee
            create_tx(45, 2000, 2.0),  // In 90-day window
            create_tx(20, 3000, 3.0),  // In 30-day window
            create_tx(5, 5000, 5.0),   // In 7-day window
            create_tx(1, 8000, 8.0),   // Newest, max fee
        ];

        let metrics = Metrics::from(transactions);

        // --- Check top-level metrics ---
        assert_eq!(
            metrics.first_transaction.unwrap().date_naive(),
            (Utc::now() - Duration::days(100)).date_naive()
        );
        assert_eq!(
            metrics.last_transaction.unwrap().date_naive(),
            (Utc::now() - Duration::days(1)).date_naive()
        );

        // --- Check all_time period ---
        let all_time = &metrics.all_time;
        assert_eq!(all_time.total_transactions, 5);
        assert_eq!(all_time.total_sponsored_amount, 19000); // 1+2+3+5+8
        assert_eq!(all_time.min_transaction_fee, 1000);
        assert_eq!(all_time.max_transaction_fee, 8000);
        assert_eq!(all_time.average_transaction_fee, 3800.0); // 19000 / 5
        let expected_daily_avg = 5.0
            / ((Utc::now() - Duration::days(1)).date_naive()
                - (Utc::now() - Duration::days(100)).date_naive())
            .num_days() as f64;
        assert!((all_time.average_daily_transactions - expected_daily_avg).abs() < 1e-9);

        // --- Check last_90_days period ---
        let last_90 = &metrics.last_90_days;
        assert_eq!(last_90.total_transactions, 4);
        assert_eq!(last_90.total_sponsored_amount, 18000); // 2+3+5+8
        assert_eq!(last_90.min_transaction_fee, 2000);
        assert_eq!(last_90.max_transaction_fee, 8000);
        assert_eq!(last_90.average_transaction_fee, 4500.0); // 18000 / 4

        // --- Check last_30_days period ---
        let last_30 = &metrics.last_30_days;
        assert_eq!(last_30.total_transactions, 3);
        assert_eq!(last_30.total_sponsored_amount, 16000); // 3+5+8
        assert_eq!(last_30.min_transaction_fee, 3000);
        assert_eq!(last_30.max_transaction_fee, 8000);
        assert_eq!(last_30.average_transaction_fee.round(), 5333.0); // 16000 / 3

        // --- Check last_7_days period ---
        let last_7 = &metrics.last_7_days;
        assert_eq!(last_7.total_transactions, 2);
        assert_eq!(last_7.total_sponsored_amount, 13000); // 5+8
        assert_eq!(last_7.min_transaction_fee, 5000);
        assert_eq!(last_7.max_transaction_fee, 8000);
        assert_eq!(last_7.average_transaction_fee, 6500.0); // 13000 / 2
    }
}
