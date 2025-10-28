use super::{ClientUpdate, GroupUpdate, TokenBalanceUpdate};
use crate::operations::get_shared_balance;
use application::views::{client::ClientView, group::GroupView};
use async_graphql::Subscription;
use futures_util::{Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct SubscriptionRoot {
    client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
    group_query_receiver: Arc<broadcast::Receiver<GroupView>>,
}

impl SubscriptionRoot {
    pub fn new(
        client_query_receiver: Arc<broadcast::Receiver<ClientView>>,
        group_query_receiver: Arc<broadcast::Receiver<GroupView>>,
    ) -> Self {
        Self {
            client_query_receiver,
            group_query_receiver,
        }
    }
}

#[Subscription]
impl SubscriptionRoot {
    /// Subscribe to token balance updates
    async fn token_balance_updates(&self) -> impl Stream<Item = TokenBalanceUpdate> {
        let balance = get_shared_balance().read().await;
        let receiver = balance.sender.subscribe();

        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .filter_map(|result| async move { result.ok() })
    }

    /// Subscribe to client updates
    async fn client_updates(&self) -> impl Stream<Item = ClientUpdate> {
        let receiver = self.client_query_receiver.resubscribe();
        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .filter_map(|result| async move { result.map(|view| view.into()).ok() })
    }

    // Subscribe to group updates
    async fn group_updates(&self) -> impl Stream<Item = GroupUpdate> {
        let receiver = self.group_query_receiver.resubscribe();
        tokio_stream::wrappers::BroadcastStream::new(receiver)
            .filter_map(|result| async move { result.map(|view| view.into()).ok() })
    }
}
