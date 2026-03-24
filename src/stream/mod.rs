pub mod yellowstone_client;
pub mod websocket_client;
pub mod polling_client;

// Use polling client by default (works with any RPC endpoint)
pub use polling_client::{
    subscribe_transactions, StreamError, TransactionUpdate, OurTransaction,
};
