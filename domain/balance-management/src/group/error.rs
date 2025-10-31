#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("Group not found")]
    GroupNotFound,
}
