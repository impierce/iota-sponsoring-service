#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Client not found")]
    ClientNotFound,
}
