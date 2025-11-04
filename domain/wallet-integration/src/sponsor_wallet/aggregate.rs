use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};

use super::{
    command::SponsorWalletCommand,
    error::SponsorWalletError,
    event::SponsorWalletEvent::{self, *},
};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SponsorWallet {
    /// The unique identifier for the sponsor wallet
    pub sponsor_wallet_id: String,
    pub address: String,
    pub balance: u64,
}

#[async_trait]
impl Aggregate for SponsorWallet {
    type Command = SponsorWalletCommand;
    type Event = SponsorWalletEvent;
    type Error = SponsorWalletError;
    type Services = ();

    fn aggregate_type() -> String {
        "sponsor_wallet".to_string()
    }

    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use SponsorWalletCommand::*;
        use SponsorWalletError::*;

        // --- Validation before processing the command ---
        let has_been_created = !self.sponsor_wallet_id.is_empty();

        match command {
            CreateSponsorWallet {
                sponsor_wallet_id,
                address,
                balance,
            } => Ok(vec![SponsorWalletCreated {
                sponsor_wallet_id,
                address,
                balance,
            }]),

            // For all other commands, the wallet must exist first.
            _ if !has_been_created => Err(SponsorWalletNotFound),

            RecordBalanceUpdate { new_balance } => {
                println!("Recording balance update to {}", new_balance);

                Ok(vec![BalanceUpdateRecorded { new_balance }])
            }

            // TODO: remove this command
            RecordTransactionFeePaid {
                sender_address,
                transaction_fee,
            } => {
                // TODO: Handle potential underflow
                let new_balance = self.balance.saturating_sub(transaction_fee);

                Ok(vec![TransactionFeePaidRecorded {
                    sender_address,
                    new_balance,
                }])
            }
        }
    }

    fn apply(&mut self, event: Self::Event) {
        use SponsorWalletEvent::*;

        match event {
            SponsorWalletCreated {
                sponsor_wallet_id,
                address,
                balance,
            } => {
                self.sponsor_wallet_id = sponsor_wallet_id;
                self.address = address;
                self.balance = balance;
            }
            BalanceUpdateRecorded { new_balance } => {
                self.balance = new_balance;
            }
            TransactionFeePaidRecorded {
                sender_address: _,
                new_balance,
            } => {
                self.balance = new_balance;
            }
        }
    }
}
