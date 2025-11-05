use uuid::Uuid;

#[derive(Debug)]
pub enum ClientCommand {
    /// Command to register a new client in the system.
    RegisterClient {
        client_id: Uuid,
        name: String,
        wallet_address: String,
    },

    /// Command to remove an existing client.
    RemoveClient { client_id: Uuid },

    /// Command to assign a client to a group.
    AssignClientToGroup { group_id: Uuid },

    /// Command to remove a client from its group.
    RemoveClientFromGroup,

    /// Command to allocate a specific balance to an individual client.
    AllocateBalanceToClient { amount: u64 },

    /// Command to withdraw balance from an individual client.
    WithdrawBalanceFromClient { amount: u64 },
}
