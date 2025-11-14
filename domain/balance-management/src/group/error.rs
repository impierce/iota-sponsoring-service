#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("Group already exists")]
    GroupAlreadyExists,
    #[error("Group not found")]
    GroupNotFound,
    #[error("Insufficient balance in group for the requested operation")]
    InsufficientBalance,
}
