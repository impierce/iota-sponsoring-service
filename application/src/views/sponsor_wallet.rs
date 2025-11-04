use cqrs_es::{Aggregate, EventEnvelope, View};
use serde::{Deserialize, Serialize};
use wallet_integration::sponsor_wallet::aggregate::SponsorWallet;

pub const SPONSOR_WALLET_VIEW_ID: &str = "sponsor-wallet";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SponsorWalletView(SponsorWallet);

impl SponsorWalletView {
    pub fn into_inner(self) -> SponsorWallet {
        self.0
    }
}

impl std::ops::Deref for SponsorWalletView {
    type Target = SponsorWallet;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for SponsorWalletView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl View<SponsorWallet> for SponsorWalletView {
    fn update(&mut self, event: &EventEnvelope<SponsorWallet>) {
        println!("Updating SponsorWalletView with event: {:?}", event.payload);

        self.0.apply(event.payload.clone());
    }
}
