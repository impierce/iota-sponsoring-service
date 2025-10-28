pub mod mutations;
pub mod queries;
pub mod subscriptions;

use application::views::client::ClientView;
use application::views::group::GroupView;
use async_graphql::SimpleObject;
use rand::Rng;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{Duration, Instant};

#[derive(Clone, SimpleObject)]
pub struct ClientUpdate {
    pub id: String,
    pub name: String,
    pub wallet_address: String,
    pub balance: Option<u64>,
    pub group_id: Option<String>,
}

impl From<ClientView> for ClientUpdate {
    fn from(view: ClientView) -> Self {
        let inner = view.into_inner();
        Self {
            id: inner.id,
            name: inner.name,
            wallet_address: inner.wallet_address,
            balance: inner.balance,
            group_id: inner.group_id,
        }
    }
}

#[derive(Clone, SimpleObject)]
pub struct GroupUpdate {
    pub id: String,
    pub name: String,
    pub balance: u64,
    pub members: HashSet<String>,
}

impl From<GroupView> for GroupUpdate {
    fn from(view: GroupView) -> Self {
        let inner = view.into_inner();
        Self {
            id: inner.id,
            name: inner.name,
            balance: inner.balance,
            members: inner.members,
        }
    }
}

#[derive(Clone, SimpleObject)]
pub struct TokenBalanceUpdate {
    pub balance: i32,
    pub timestamp: String,
    pub action: String, // "decreased" or "reset"
}

#[derive(Clone)]
pub struct TokenBalance {
    value: i32,
    last_updated: Instant,
    sender: broadcast::Sender<TokenBalanceUpdate>,
}

impl TokenBalance {
    fn new() -> (Self, broadcast::Receiver<TokenBalanceUpdate>) {
        let (sender, receiver) = broadcast::channel(100);
        let balance = Self {
            value: 50000,
            last_updated: Instant::now(),
            sender,
        };
        (balance, receiver)
    }

    fn update(&mut self) {
        let decrease = rand::rng().random_range(100..=2000);
        let (action, new_value) = if self.value <= decrease {
            ("reset".to_string(), 50000)
        } else {
            ("decreased".to_string(), self.value - decrease)
        };

        self.value = new_value;
        self.last_updated = Instant::now();

        let update = TokenBalanceUpdate {
            balance: self.value,
            timestamp: chrono::Utc::now().to_rfc3339(),
            action,
        };

        println!("Token balance {}: {}", update.action, self.value);
        let _ = self.sender.send(update);
    }
}

// Shared state using std::sync::OnceLock (no need for lazy_static)
static SHARED_BALANCE: std::sync::OnceLock<Arc<RwLock<TokenBalance>>> = std::sync::OnceLock::new();

fn get_shared_balance() -> &'static Arc<RwLock<TokenBalance>> {
    SHARED_BALANCE.get_or_init(|| {
        let (balance, _) = TokenBalance::new();
        let shared = Arc::new(RwLock::new(balance));

        // Start background task to update balance with random interval (2-5 seconds)
        let balance_clone = shared.clone();
        tokio::spawn(async move {
            loop {
                // Random interval between 2-5 seconds
                let random_seconds = rand::rng().random_range(2..=5);
                tokio::time::sleep(Duration::from_secs(random_seconds)).await;

                let mut balance = balance_clone.write().await;
                balance.update();
            }
        });

        shared
    })
}
