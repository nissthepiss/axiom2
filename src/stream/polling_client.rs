use futures::{stream::Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error as StdError;
use std::time::Duration;
use thiserror::Error;
use tokio::time::interval;

/// Solana RPC endpoint
const DEFAULT_RPC_HOST: &str = "http://localhost:8899";

/// Load credentials from environment variables
fn load_chainstack_credentials() -> (String, String) {
    let username = std::env::var("CHAINSTACK_USERNAME")
        .unwrap_or_else(|_| "loving-jepsen".to_string());

    let password = std::env::var("CHAINSTACK_PASSWORD")
        .unwrap_or_else(|_| "haven-shiny-purple-perch-tacky-ramp".to_string());

    (username, password)
}

#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Subscription failed: {0}")]
    SubscriptionError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("RPC error: {0}")]
    RpcError(String),
}

impl From<anyhow::Error> for StreamError {
    fn from(e: anyhow::Error) -> Self {
        StreamError::ConnectionError(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Clone, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct SlotInfo {
    slot: u64,
}

/// Raw polling message type (for API compatibility)
#[derive(Debug, Clone)]
pub struct RawWebSocketMessage {
    pub slot: u64,
    pub is_transaction: bool,
}

/// Our simplified transaction update type
pub struct TransactionUpdate {
    pub slot: u64,
    pub transaction: Option<OurTransaction>,
}

impl TransactionUpdate {
    pub fn from_update(update: &RawWebSocketMessage) -> Option<Self> {
        Some(TransactionUpdate {
            slot: update.slot,
            transaction: None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OurTransaction {
    pub signatures: Vec<Vec<u8>>,
    pub message: Vec<u8>,
    pub meta: Option<TransactionMeta>,
}

#[derive(Debug, Clone)]
pub struct TransactionMeta {
    pub error: Option<Vec<u8>>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_index: u32,
    pub mint: Vec<u8>,
    pub ui_token_amount: UiTokenAmount,
    pub owner: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct UiTokenAmount {
    pub amount: String,
    pub decimals: u32,
    pub ui_amount_string: String,
    pub ui_amount: f64,
}

/// Subscribe to Solana RPC slot updates via polling
pub async fn subscribe_transactions(
    endpoint: &str,
) -> Result<impl Stream<Item = Result<RawWebSocketMessage, StreamError>>, StreamError> {
    // Determine the RPC endpoint
    let rpc_url = if endpoint.contains("chainstack") || endpoint.contains("yellowstone-solana-mainnet") {
        // Chainstack RPC endpoint (not gRPC)
        "https://solana-mainnet.core.chainstack.com".to_string()
    } else if endpoint.is_empty() || endpoint == "default" {
        DEFAULT_RPC_HOST.to_string()
    } else {
        endpoint.to_string()
    };

    println!("Connecting to Solana RPC at: {}", rpc_url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| StreamError::ConnectionError(format!("Failed to create HTTP client: {}", e)))?;

    println!("RPC client created");

    Ok(async_stream::stream! {
        let mut last_slot = 0u64;
        let mut ticker = interval(Duration::from_secs(1));
        let client_clone = client.clone();
        let url = rpc_url.clone();

        loop {
            ticker.tick().await;

            // Get the current slot
            let request = RpcRequest {
                jsonrpc: "2.0".to_string(),
                id: 1,
                method: "getSlot".to_string(),
                params: json!([]),
            };

            match send_rpc_request(&client_clone, &url, &request).await {
                Ok(slot_response) => {
                    if let Some(slot) = parse_slot_response(&slot_response) {
                        if slot > last_slot {
                            println!("slot: {}", slot);
                            last_slot = slot;
                            yield Ok(RawWebSocketMessage {
                                slot,
                                is_transaction: false,
                            });
                        }
                    }
                }
                Err(e) => {
                    yield Err(StreamError::RpcError(format!("Failed to get slot: {}", e)));
                }
            }
        }
    })
}

async fn send_rpc_request(
    client: &reqwest::Client,
    url: &str,
    request: &RpcRequest,
) -> Result<RpcResponse, StreamError> {
    let mut req_builder = client.post(url).json(request);

    // Add basic auth for Chainstack
    if url.contains("chainstack") {
        let (username, password) = load_chainstack_credentials();
        req_builder = req_builder.basic_auth(username, Some(password));
    }

    let response = req_builder
        .send()
        .await
        .map_err(|e| StreamError::ConnectionError(format!("HTTP request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(StreamError::RpcError(format!("HTTP status: {}", response.status())));
    }

    let rpc_response: RpcResponse = response
        .json()
        .await
        .map_err(|e| StreamError::ParseError(format!("Failed to parse response: {}", e)))?;

    if let Some(error) = rpc_response.error {
        return Err(StreamError::RpcError(format!("RPC error {}: {}", error.code, error.message)));
    }

    Ok(rpc_response)
}

fn parse_slot_response(response: &RpcResponse) -> Option<u64> {
    if let Some(result) = &response.result {
        if let Some(slot) = result.as_u64() {
            return Some(slot);
        }
    }
    None
}
