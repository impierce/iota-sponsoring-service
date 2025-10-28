pub enum GroupCommand {
    /// Command to create a new group.
    CreateGroup { id: String, name: String },

    /// Command to delete an existing group.
    DeleteGroup { id: String },

    /// Command to add a client to a group.
    AddClientToGroup { client_id: String },

    /// Command to remove a client from a group.
    RemoveClientFromGroup { client_id: String },

    /// Command to allocate a shared balance to a group.
    AllocateBalanceToGroup { id: String, amount: u64 },

    /// Command to withdraw from a group's shared balance.
    WithdrawBalanceFromGroup { id: String, amount: u64 },
}
