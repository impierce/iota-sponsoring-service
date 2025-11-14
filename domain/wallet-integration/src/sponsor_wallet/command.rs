use url::Url;

#[derive(Debug)]
pub enum SponsorWalletCommand {
    CreateSponsorWallet {
        sponsor_wallet_id: String,
        address: String,
        balance: u64,
    },

    UpdateSponsorWalletName {
        name: String,
    },

    UpdateSponsorWalletLogoUri {
        logo_uri: Option<Url>,
    },

    RecordBalanceUpdate {
        new_balance: u64,
    },
}
