use uuid::Uuid;

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
    AllocateBalanceToGroup { group_id: Uuid, amount: u64 },

    /// Command to withdraw from a group's shared balance.
    WithdrawBalanceFromGroup { group_id: Uuid, amount: u64 },
}
