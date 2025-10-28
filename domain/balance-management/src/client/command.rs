pub enum ClientCommand {
    /// Command to register a new client in the system.
    RegisterClient {
        id: String,
        name: String,
        wallet_address: String,
    },

    /// Command to remove an existing client.
    RemoveClient { id: String },

    /// Command to assign a client to a group.
    AssignClientToGroup { group_id: String },

    /// Command to remove a client from its group.
    RemoveClientFromGroup,

    /// Command to allocate a specific balance to an individual client.
    AllocateBalanceToClient { amount: u64 },

    /// Command to withdraw balance from an individual client.
    WithdrawBalanceFromClient { amount: u64 },
}
