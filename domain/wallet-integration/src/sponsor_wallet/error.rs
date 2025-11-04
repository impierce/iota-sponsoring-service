#[derive(Debug, thiserror::Error)]
pub enum SponsorWalletError {
    #[error("Sponsor wallet not found")]
    SponsorWalletNotFound,
}
