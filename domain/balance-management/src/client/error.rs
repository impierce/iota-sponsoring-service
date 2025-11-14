#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Client already exists")]
    ClientAlreadyExists,
    #[error("Client not found")]
    ClientNotFound,
    #[error("Insufficient balance for withdrawal")]
    InsufficientBalance,
}
