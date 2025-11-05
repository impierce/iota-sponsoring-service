use uuid::Uuid;

#[derive(Debug)]
pub enum GroupCommand {
    /// Command to create a new group.
    CreateGroup { group_id: Uuid, name: String },

    /// Command to delete an existing group.
    DeleteGroup { group_id: Uuid },

    /// Command to add a client to a group.
    AddClientToGroup { client_id: Uuid },

    /// Command to remove a client from a group.
    RemoveClientFromGroup { client_id: Uuid },

    /// Command to allocate a shared balance to a group.
    AllocateFundsToGroup {
        // TODO: remove `group_id` from here, as it's already part of the aggregate state
        group_id: Uuid,
        amount: u64,
    },

    /// Command to withdraw from a group's shared balance.
    WithdrawFundsFromGroup {
        // TODO: remove `group_id` from here, as it's already part of the aggregate state
        group_id: Uuid,
        amount: u64,
    },

    /// Command to record a transaction fee paid by the group.
    RecordTransactionFeePaid { transaction_fee: u64 },
}
