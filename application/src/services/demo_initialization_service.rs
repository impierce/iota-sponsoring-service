use std::sync::Arc;

use anyhow::Result;
use balance_management::{
    client::{aggregate::Client, command::ClientCommand},
    group::{aggregate::Group, command::GroupCommand},
};
use chrono::{DateTime, Duration, Utc};
use cqrs_es::{CqrsFramework, EventStore, persist::ViewRepository};
use rand::Rng;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

use crate::{
    services::allocation_service::{get_iota_eur_price, get_iota_usd_price},
    views::{
        client::ClientView, client_list::ClientListView, group::GroupView,
        group_list::GroupListView, sponsor_wallet::SponsorWalletView,
    },
};
pub struct DemoInitializationService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet>,
    CES: EventStore<Client>,
    GES: EventStore<Group>,
{
    _sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
    client_handler: Arc<CqrsFramework<Client, CES>>,
    group_handler: Arc<CqrsFramework<Group, GES>>,
    _sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
    _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
    _client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
    group_view: Arc<dyn ViewRepository<GroupView, Group>>,
    _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
}

impl<SWES, CES, GES> DemoInitializationService<SWES, CES, GES>
where
    SWES: EventStore<SponsorWallet> + 'static,
    CES: EventStore<Client> + 'static,
    GES: EventStore<Group> + 'static,
{
    pub fn new(
        _sponsor_wallet_handler: Arc<CqrsFramework<SponsorWallet, SWES>>,
        client_handler: Arc<CqrsFramework<Client, CES>>,
        group_handler: Arc<CqrsFramework<Group, GES>>,
        _sponsor_wallet_view: Arc<dyn ViewRepository<SponsorWalletView, SponsorWallet>>,
        _client_view: Arc<dyn ViewRepository<ClientView, Client>>,
        _client_list_view: Arc<dyn ViewRepository<ClientListView, Client>>,
        group_view: Arc<dyn ViewRepository<GroupView, Group>>,
        _group_list_view: Arc<dyn ViewRepository<GroupListView, Group>>,
    ) -> Self {
        Self {
            _sponsor_wallet_handler,
            client_handler,
            group_handler,
            _sponsor_wallet_view,
            _client_view,
            _client_list_view,
            group_view,
            _group_list_view,
        }
    }

    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing demo data...");

        self.initialize_evc_ecosystem().await?;
        self.initialize_surf_ecosystem().await?;
        self.initialize_tech_partners().await?;

        info!("Demo data initialization completed.");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn initialize_evc_ecosystem(&self) -> Result<()> {
        info!("Initializing demo data...");

        let name = "EVC Ecosystem".to_string();
        let group_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());

        // Check if the demo group already exists
        if let Some(_) = self.group_view.load(&group_id.to_string()).await? {
            warn!("`{name}` demo group already exists. Skipping demo data initialization.");
            return Ok(());
        }

        let logo_uri = Some(
            url::Url::parse("https://evc-nederland.nl/wp-content/uploads/2023/11/evc-nederland_logo-website-1.png")
                .unwrap(),
        );

        info!("Creating new group");
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
            logo_uri,
        };

        debug!("Dispatching `CreateGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let command = GroupCommand::AllocateFundsToGroup {
            group_id,
            amount: 10_000_000_000,
        };

        debug!("Dispatching `AllocateFundsToGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let input = vec![
            (
                "EVC Nederland".to_string(),
                None,
                "0xdemoaddressA".to_string(),
                90,
            ),
            (
                "Libereaux".to_string(),
                None,
                "0xdemoaddressB".to_string(),
                60,
            ),
            (
                "ExamenKamer".to_string(),
                None,
                "0xdemoaddressC".to_string(),
                30,
            ),
        ];

        info!("Adding demo clients to the demo group");
        for (name, logo_uri, wallet_address, since) in input {
            let client_id = uuid::Uuid::new_v4();

            let command = ClientCommand::RegisterClient {
                client_id: client_id.clone(),
                name: name.clone(),
                logo_uri,
                website_uri: None,
                wallet_address,
            };

            debug!("Dispatching `RegisterClient` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = ClientCommand::AssignClientToGroup {
                group_id: group_id.clone(),
            };

            debug!("Dispatching `AssignClientToGroup` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = GroupCommand::AddClientToGroup { client_id };

            debug!("Dispatching `AddClientToGroup` command");
            self.group_handler
                .execute(&group_id.to_string(), command)
                .await?;

            let transactions = generate_random_transactions(50, since);

            let iota_eur_price = get_iota_usd_price().await;
            let iota_usd_price = get_iota_eur_price().await;

            for (timestamp, transaction_fee) in transactions {
                let transaction_fee_iot = transaction_fee as f64 / 1_000_000_000.0;

                let transaction_fee_eur =
                    iota_eur_price * (transaction_fee as f64 / 1_000_000_000.0);

                let transaction_fee_usd =
                    iota_usd_price * (transaction_fee as f64 / 1_000_000_000.0);

                let command = GroupCommand::RecordTransactionFeePaidForDemo {
                    client_id: client_id.clone(),
                    client_name: name.clone(),
                    transaction_fee,
                    transaction_fee_iot,
                    transaction_fee_eur,
                    transaction_fee_usd,
                    timestamp,
                };

                debug!("Dispatching `RecordTransactionFeePaidForDemo` command");
                self.group_handler
                    .execute(&group_id.to_string(), command)
                    .await?;
            }
        }

        info!("Demo data initialization completed.");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn initialize_surf_ecosystem(&self) -> Result<()> {
        info!("Initializing demo data...");

        let name = "SURF Ecosystem".to_string();
        let group_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());

        // Check if the demo group already exists
        if let Some(_) = self.group_view.load(&group_id.to_string()).await? {
            warn!("`{name}` demo group already exists. Skipping demo data initialization.");
            return Ok(());
        }

        let logo_uri = Some(
            url::Url::parse(
                "https://images.seeklogo.com/logo-png/51/2/surfnet-logo-png_seeklogo-515342.png",
            )
            .unwrap(),
        );

        info!("Creating new group");
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
            logo_uri,
        };

        debug!("Dispatching `CreateGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let command = GroupCommand::AllocateFundsToGroup {
            group_id,
            amount: 10_000_000_000,
        };

        debug!("Dispatching `AllocateFundsToGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let input = vec![
            (
                "Rijksuniversiteit Groningen".to_string(),
                None,
                "0xdemoaddressD".to_string(),
                90,
            ),
            ("Avans".to_string(), None, "0xdemoaddressE".to_string(), 60),
            (
                "Koning Willem I College".to_string(),
                None,
                "0xdemoaddressF".to_string(),
                30,
            ),
        ];

        info!("Adding demo clients to the demo group");
        for (name, logo_uri, wallet_address, since) in input {
            let client_id = uuid::Uuid::new_v4();

            let command = ClientCommand::RegisterClient {
                client_id: client_id.clone(),
                name: name.clone(),
                logo_uri,
                website_uri: None,
                wallet_address,
            };

            debug!("Dispatching `RegisterClient` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = ClientCommand::AssignClientToGroup {
                group_id: group_id.clone(),
            };

            debug!("Dispatching `AssignClientToGroup` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = GroupCommand::AddClientToGroup { client_id };

            debug!("Dispatching `AddClientToGroup` command");
            self.group_handler
                .execute(&group_id.to_string(), command)
                .await?;

            let transactions = generate_random_transactions(50, since);

            let iota_eur_price = get_iota_usd_price().await;
            let iota_usd_price = get_iota_eur_price().await;

            for (timestamp, transaction_fee) in transactions {
                let transaction_fee_iot = transaction_fee as f64 / 1_000_000_000.0;

                let transaction_fee_eur =
                    iota_eur_price * (transaction_fee as f64 / 1_000_000_000.0);

                let transaction_fee_usd =
                    iota_usd_price * (transaction_fee as f64 / 1_000_000_000.0);

                let command = GroupCommand::RecordTransactionFeePaidForDemo {
                    client_id: client_id.clone(),
                    client_name: name.clone(),
                    transaction_fee,
                    transaction_fee_iot,
                    transaction_fee_eur,
                    transaction_fee_usd,
                    timestamp,
                };

                debug!("Dispatching `RecordTransactionFeePaidForDemo` command");
                self.group_handler
                    .execute(&group_id.to_string(), command)
                    .await?;
            }
        }

        info!("Demo data initialization completed.");
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn initialize_tech_partners(&self) -> Result<()> {
        info!("Initializing demo data...");

        let name = "Tech Partners".to_string();
        let group_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, name.as_bytes());

        // Check if the demo group already exists
        if let Some(_) = self.group_view.load(&group_id.to_string()).await? {
            warn!("`{name}` demo group already exists. Skipping demo data initialization.");
            return Ok(());
        }

        let logo_uri = Some(
            url::Url::parse("https://d315pvdvxi2gex.cloudfront.net/d96a337f84c5c900f31e08818.png")
                .unwrap(),
        );

        info!("Creating new group");
        let command = GroupCommand::CreateGroup {
            group_id: group_id.clone(),
            name,
            logo_uri,
        };

        debug!("Dispatching `CreateGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let command = GroupCommand::AllocateFundsToGroup {
            group_id,
            amount: 10_000_000_000,
        };

        debug!("Dispatching `AllocateFundsToGroup` command");
        self.group_handler
            .execute(&group_id.to_string(), command)
            .await?;

        let input = vec![
            (
                "Filancore".to_string(),
                None,
                "0xdemoaddressG".to_string(),
                90,
            ),
            (
                "IOTA Foundation".to_string(),
                None,
                "0xdemoaddressH".to_string(),
                60,
            ),
            (
                "TWIN Foundation".to_string(),
                None,
                "0xdemoaddressI".to_string(),
                30,
            ),
        ];

        info!("Adding demo clients to the demo group");
        for (name, logo_uri, wallet_address, since) in input {
            let client_id = uuid::Uuid::new_v4();

            let command = ClientCommand::RegisterClient {
                client_id: client_id.clone(),
                name: name.clone(),
                logo_uri,
                website_uri: None,
                wallet_address,
            };

            debug!("Dispatching `RegisterClient` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = ClientCommand::AssignClientToGroup {
                group_id: group_id.clone(),
            };

            debug!("Dispatching `AssignClientToGroup` command");
            self.client_handler
                .execute(&client_id.to_string(), command)
                .await?;

            let command = GroupCommand::AddClientToGroup { client_id };

            debug!("Dispatching `AddClientToGroup` command");
            self.group_handler
                .execute(&group_id.to_string(), command)
                .await?;

            let transactions = generate_random_transactions(50, since);

            let iota_eur_price = get_iota_usd_price().await;
            let iota_usd_price = get_iota_eur_price().await;

            for (timestamp, transaction_fee) in transactions {
                let transaction_fee_iot = transaction_fee as f64 / 1_000_000_000.0;

                let transaction_fee_eur =
                    iota_eur_price * (transaction_fee as f64 / 1_000_000_000.0);

                let transaction_fee_usd =
                    iota_usd_price * (transaction_fee as f64 / 1_000_000_000.0);

                let command = GroupCommand::RecordTransactionFeePaidForDemo {
                    client_id: client_id.clone(),
                    client_name: name.clone(),
                    transaction_fee,
                    transaction_fee_iot,
                    transaction_fee_eur,
                    transaction_fee_usd,
                    timestamp,
                };

                debug!("Dispatching `RecordTransactionFeePaidForDemo` command");
                self.group_handler
                    .execute(&group_id.to_string(), command)
                    .await?;
            }
        }

        info!("Demo data initialization completed.");
        Ok(())
    }
}

fn generate_random_transactions(n: usize, since: i64) -> Vec<(DateTime<Utc>, u64)> {
    let mut rng = rand::rng();
    let now = Utc::now();
    let ninety_days = Duration::days(since).num_seconds();

    let mut transactions: Vec<(DateTime<Utc>, u64)> = (0..n)
        .map(|_| {
            // Random seconds offset between 0 and 90 days
            let seconds_ago = rng.random_range(0..=ninety_days);
            let dt = now - Duration::seconds(seconds_ago);
            // Random fee between 1_000_000 and 10_000_000
            let fee = rng.random_range(1_000_000..=10_000_000);
            (dt, fee)
        })
        .collect();

    // Sort transactions by DateTime
    transactions.sort_by_key(|k| k.0);

    transactions
}
