use std::sync::Arc;

use application::{
    queries::{generic_query_with_sender::GenericQueryWithSender, list_all_query::ListAllQuery},
    services::{
        allocation_service::AllocationService,
        authorize_transaction_service::{self, AuthorizeTransactionService},
        balance_management_service::BalanceManagementService,
    },
    views::{
        client::ClientView,
        client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
        group::GroupView,
        group_list::{GROUP_LIST_VIEW_ID, GroupListView},
        sponsor_wallet::{SPONSOR_WALLET_VIEW_ID, SponsorWalletView},
    },
};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use cqrs_es::persist::ViewRepository;
use iota_config::Config;
use iota_gas_station::storage::connect_storage;
use iota_gas_station::{config::GasStationConfig, metrics::StorageMetrics};
use mongo_es::{MongoCqrs, MongoEventRepository, MongoViewRepository, default_mongo_client};
use tokio::sync::broadcast;
use wallet_integration::sponsor_wallet::{aggregate::SponsorWallet, command::SponsorWalletCommand};

pub struct CompositionRoot {
    pub allocation_service: Arc<
        AllocationService<
            PersistedEventStore<MongoEventRepository, SponsorWallet>,
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
    pub authorize_transaction_service: Arc<
        AuthorizeTransactionService<
            PersistedEventStore<MongoEventRepository, SponsorWallet>,
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
    pub balance_management_service: Arc<
        BalanceManagementService<
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,

    pub sponsor_wallet_view: Arc<MongoViewRepository<SponsorWalletView, SponsorWallet>>,
    pub client_view: Arc<MongoViewRepository<ClientView, Client>>,
    pub client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
    pub group_view: Arc<MongoViewRepository<GroupView, Group>>,
    pub group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,

    pub sponsor_wallet_query_receiver: Arc<broadcast::Receiver<SponsorWalletView>>,
    pub client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
    pub group_query_receiver: Arc<broadcast::Receiver<GroupView>>,
}

async fn initialize_sponsor_wallet() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

impl CompositionRoot {
    pub async fn new(mongo_uri: String, gas_station_config_path: String) -> Self {
        let client = default_mongo_client(&mongo_uri).await;

        let sponsor_wallet_view =
            Arc::new(MongoViewRepository::new("sponsor_wallet", client.clone()));
        let sponsor_wallet_event_repository =
            MongoEventRepository::new(client.clone()).await.unwrap();
        let sponsor_wallet_event_store =
            PersistedEventStore::new_event_store(sponsor_wallet_event_repository);
        let sponsor_wallet_channel = broadcast::channel(100);
        let sponsor_wallet_query =
            GenericQueryWithSender::new(sponsor_wallet_view.clone(), sponsor_wallet_channel.0);
        let sponsor_wallet_query_receiver = Arc::new(sponsor_wallet_channel.1);
        let sponsor_wallet_handler = Arc::new(MongoCqrs::new(
            sponsor_wallet_event_store,
            vec![Box::new(sponsor_wallet_query)],
            (),
        ));

        let GasStationConfig {
            signer_config,
            // storage_config: gas_station_config,
            ..
        } = GasStationConfig::load(&gas_station_config_path).expect("Failed to load config file");

        if sponsor_wallet_view
            .load(SPONSOR_WALLET_VIEW_ID)
            .await
            .unwrap()
            .is_none()
        {
            let signer = signer_config.new_signer().await;
            let sponsor_address = signer.get_address();

            println!("Creating sponsor wallet view with address: {sponsor_address}");

            let command = SponsorWalletCommand::CreateSponsorWallet {
                sponsor_wallet_id: SPONSOR_WALLET_VIEW_ID.to_string(),
                address: sponsor_address.to_string(),
                balance: 0,
            };

            sponsor_wallet_handler
                .execute(SPONSOR_WALLET_VIEW_ID, command)
                .await
                .unwrap();
        } else {
            println!("Sponsor wallet view already exists");
        }

        let client_view = Arc::new(MongoViewRepository::new("client", client.clone()));
        let client_list_view: Arc<MongoViewRepository<ClientListView, Client>> = Arc::new(
            MongoViewRepository::new(CLIENT_LIST_VIEW_ID, client.clone()),
        );
        let client_event_repository = MongoEventRepository::new(client.clone()).await.unwrap();
        let client_event_store = PersistedEventStore::new_event_store(client_event_repository);
        let client_channel = broadcast::channel(100);
        let client_query = GenericQueryWithSender::new(client_view.clone(), client_channel.0);
        let client_list_query = ListAllQuery::new(client_list_view.clone(), CLIENT_LIST_VIEW_ID);
        let client_query_receiver = Arc::new(client_channel.1);
        let client_handler = Arc::new(MongoCqrs::new(
            client_event_store,
            vec![Box::new(client_query), Box::new(client_list_query)],
            (),
        ));

        let group_view: Arc<MongoViewRepository<GroupView, Group>> =
            Arc::new(MongoViewRepository::new("group", client.clone()));
        let group_list_view: Arc<MongoViewRepository<GroupListView, Group>> =
            Arc::new(MongoViewRepository::new(GROUP_LIST_VIEW_ID, client.clone()));
        let group_event_repository = MongoEventRepository::new(client.clone()).await.unwrap();
        let group_event_store = PersistedEventStore::new_event_store(group_event_repository);
        let group_channel = broadcast::channel(100);
        let group_query = GenericQueryWithSender::new(group_view.clone(), group_channel.0);
        let group_list_query = ListAllQuery::new(group_list_view.clone(), GROUP_LIST_VIEW_ID);
        let group_query_receiver = Arc::new(group_channel.1);
        let group_handler = Arc::new(MongoCqrs::new(
            group_event_store,
            vec![Box::new(group_query), Box::new(group_list_query)],
            (),
        ));

        let allocation_service = Arc::new(AllocationService::new(
            sponsor_wallet_handler.clone(),
            client_handler.clone(),
            group_handler.clone(),
            sponsor_wallet_view.clone(),
            client_view.clone(),
            client_list_view.clone(),
            group_view.clone(),
            group_list_view.clone(),
        ));

        let authorize_transaction_service = Arc::new(AuthorizeTransactionService::new(
            sponsor_wallet_handler,
            client_handler.clone(),
            group_handler.clone(),
            sponsor_wallet_view.clone(),
            client_view.clone(),
            client_list_view.clone(),
            group_view.clone(),
            group_list_view.clone(),
        ));

        let balance_management_service = Arc::new(BalanceManagementService::new(
            client_handler,
            group_handler,
            client_view.clone(),
            client_list_view.clone(),
            group_view.clone(),
            group_list_view.clone(),
        ));

        Self {
            allocation_service,
            authorize_transaction_service,
            balance_management_service,
            sponsor_wallet_view,
            client_view,
            client_list_view,
            group_view,
            group_list_view,
            sponsor_wallet_query_receiver,
            client_query_receiver,
            group_query_receiver,
        }
    }
}
