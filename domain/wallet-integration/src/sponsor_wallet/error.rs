#[derive(Debug, thiserror::Error)]
pub enum SponsorWalletError {
    #[error("Sponsor wallet already exists")]
    SponsorWalletAlreadyExists,
    #[error("Sponsor wallet not found")]
    SponsorWalletNotFound,
}
