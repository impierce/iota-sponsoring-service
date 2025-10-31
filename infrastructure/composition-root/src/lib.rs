use std::sync::Arc;

use application::{
    queries::{generic_query_with_sender::GenericQueryWithSender, list_all_query::ListAllQuery},
    services::balance_management_service::BalanceManagementService,
    views::{
        client::ClientView,
        client_list::{CLIENT_LIST_VIEW_ID, ClientListView},
        group::GroupView,
        group_list::{GROUP_LIST_VIEW_ID, GroupListView},
    },
};
use balance_management::{client::aggregate::Client, group::aggregate::Group};
use cqrs_es::persist::PersistedEventStore;
use mongo_es::{MongoCqrs, MongoEventRepository, MongoViewRepository, default_mongo_client};
use tokio::sync::broadcast;

pub struct CompositionRoot {
    pub balance_management_service: Arc<
        BalanceManagementService<
            PersistedEventStore<MongoEventRepository, Client>,
            PersistedEventStore<MongoEventRepository, Group>,
        >,
    >,
    pub client_view: Arc<MongoViewRepository<ClientView, Client>>,
    pub client_list_view: Arc<MongoViewRepository<ClientListView, Client>>,
    pub group_view: Arc<MongoViewRepository<GroupView, Group>>,
    pub group_list_view: Arc<MongoViewRepository<GroupListView, Group>>,
    pub client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
    pub group_query_receiver: Arc<broadcast::Receiver<GroupView>>,
}

impl CompositionRoot {
    pub async fn new(mongo_uri: String) -> Self {
        let client = default_mongo_client(&mongo_uri).await;

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
        let client_handler = MongoCqrs::new(
            client_event_store,
            vec![Box::new(client_query), Box::new(client_list_query)],
            (),
        );

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
        let group_handler = MongoCqrs::new(
            group_event_store,
            vec![Box::new(group_query), Box::new(group_list_query)],
            (),
        );

        let balance_management_service = Arc::new(BalanceManagementService::new(
            client_handler,
            group_handler,
            client_view.clone(),
            client_list_view.clone(),
            group_view.clone(),
            group_list_view.clone(),
        ));

        Self {
            balance_management_service,
            client_view,
            client_list_view,
            group_view,
            group_list_view,
            client_query_receiver,
            group_query_receiver,
        }
    }
}
