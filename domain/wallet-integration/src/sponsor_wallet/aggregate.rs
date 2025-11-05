use async_trait::async_trait;
use cqrs_es::Aggregate;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, trace};

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

    #[instrument(name = "SponsorWallet::handle", skip(self, _service), fields(command = ?command))]
    async fn handle(
        &self,
        command: Self::Command,
        _service: &Self::Services,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        use SponsorWalletCommand::*;
        use SponsorWalletError::*;

        debug!("Handling command");

        // --- Validation before processing the command ---
        let has_been_created = !self.sponsor_wallet_id.is_empty();

        match command {
            CreateSponsorWallet {
                sponsor_wallet_id,
                address,
                balance,
            } => {
                if has_been_created {
                    debug!("Validation failed: Sponsor wallet already exists");
                    return Err(SponsorWalletAlreadyExists);
                }
                Ok(vec![SponsorWalletCreated {
                    sponsor_wallet_id,
                    address,
                    balance,
                }])
            }

            // For all other commands, the wallet must exist first.
            _ if !has_been_created => {
                debug!("Validation failed: Sponsor wallet not found");
                Err(SponsorWalletNotFound)
            }

            RecordBalanceUpdate { new_balance } => {
                debug!("Recording balance update to {}", new_balance);
                Ok(vec![BalanceUpdateRecorded { new_balance }])
            }
        }
    }

    #[instrument(name = "SponsorWallet::apply", skip(self), fields(event = ?event))]
    fn apply(&mut self, event: Self::Event) {
        trace!("Applying event");
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
