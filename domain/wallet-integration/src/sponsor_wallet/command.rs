pub enum SponsorWalletCommand {
    CreateSponsorWallet {
        sponsor_wallet_id: String,
        address: String,
        balance: u64,
    },
    RecordBalanceUpdate {
        new_balance: u64,
    },
    RecordTransactionFeePaid {
        sender_address: String,
        transaction_fee: u64,
    },
}
