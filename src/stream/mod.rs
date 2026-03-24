pub mod yellowstone_client;
pub mod websocket_client;

// Use Yellowstone gRPC client (required architecture)
pub use yellowstone_client::{
    subscribe_transactions, StreamError, TransactionUpdate, OurTransaction,
    load_auth_config,
};

// Re-export the generated types
pub use yellowstone_client::geyser::{self, SubscribeUpdate, SubscribeRequest, CommitmentLevel, SubscribeRequestFilterTransactions};
