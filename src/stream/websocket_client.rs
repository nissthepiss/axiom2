use futures::{stream::Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use thiserror::Error;
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
    WebSocketStream,
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use base64::Engine as _;

/// Chainstack WebSocket endpoint
const CHAINSTACK_WS_HOST: &str = "yellowstone-solana-mainnet.core.chainstack.com";

/// Load credentials from environment variables
fn load_chainstack_credentials() -> (String, String, String) {
    let token = std::env::var("CHAINSTACK_X_TOKEN")
        .unwrap_or_else(|_| "322e4e2a19ffade6ebb982ddf1d8d62d".to_string());

    let username = std::env::var("CHAINSTACK_USERNAME")
        .unwrap_or_else(|_| "loving-jepsen".to_string());

    let password = std::env::var("CHAINSTACK_PASSWORD")
        .unwrap_or_else(|_| "haven-shiny-purple-perch-tacky-ramp".to_string());

    (token, username, password)
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
}

impl From<anyhow::Error> for StreamError {
    fn from(e: anyhow::Error) -> Self {
        StreamError::ConnectionError(e.to_string())
    }
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method")]
enum WsRequest {
    #[serde(rename = "transactionSubscribe")]
    TransactionSubscribe {
        params: TransactionSubscribeParams,
    },
    #[serde(rename = "slotSubscribe")]
    SlotSubscribe,
    #[serde(rename = "unsubscribe")]
    Unsubscribe { params: UnsubscribeParams },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionSubscribeParams {
    #[serde(rename = "transactionNotificationFilter")]
    transaction_notification_filter: TransactionNotificationFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionNotificationFilter {
    #[serde(rename = "encodeAccounts")]
    encode_accounts: Option<String>,
    #[serde(rename = "encodeTransactions")]
    encode_transactions: Option<String>,
    #[serde(rename = "includeFailedTransaction")]
    include_failed_transaction: bool,
    #[serde(rename = "commitment")]
    commitment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnsubscribeParams {
    #[serde(rename = "subscription")]
    subscription: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "message")]
enum WsResponse {
    #[serde(rename = "accountNotification")]
    AccountNotification {},
    #[serde(rename = "slotNotification")]
    SlotNotification {
        params: SlotNotificationParams,
    },
    #[serde(rename = "transactionNotification")]
    TransactionNotification {
        params: TransactionNotificationParams,
    },
    #[serde(rename = "error")]
    Error {
        params: ErrorParams,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct SlotNotificationParams {
    #[serde(rename = "result")]
    result: SlotInfo,
    subscription: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct SlotInfo {
    slot: u64,
    #[serde(rename = "timestamp")]
    timestamp: Option<u64>,
    #[serde(rename = "type")]
    slot_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TransactionNotificationParams {
    #[serde(rename = "result")]
    result: TransactionInfo,
    subscription: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct TransactionInfo {
    slot: u64,
    #[serde(rename = "transaction")]
    transaction: Option<Transaction>,
    #[serde(rename = "timestamp")]
    timestamp: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct Transaction {
    signatures: Vec<String>,
    message: String,
    meta: Option<TransactionMeta>,
}

#[derive(Debug, Clone, Deserialize)]
struct TransactionMeta {
    error: Option<String>,
    fee: u64,
    #[serde(rename = "preBalances")]
    pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    post_balances: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ErrorParams {
    #[serde(rename = "errorMessage")]
    error_message: String,
}

/// Raw WebSocket message type (for API compatibility)
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
            transaction: None, // TODO: Parse transaction from WebSocket message
        })
    }
}

#[derive(Debug, Clone)]
pub struct OurTransaction {
    pub signatures: Vec<Vec<u8>>,
    pub message: Vec<u8>,
    pub meta: Option<TransactionMeta>,
}

/// Subscribe to Chainstack WebSocket transaction stream
pub async fn subscribe_transactions(
    endpoint: &str,
) -> Result<impl Stream<Item = Result<RawWebSocketMessage, StreamError>>, StreamError> {
    // Load credentials
    let (token, username, password) = load_chainstack_credentials();

    // Determine the WebSocket endpoint
    let use_auth = endpoint.contains("chainstack") || endpoint.contains("yellowstone-solana-mainnet");
    let ws_url = if use_auth {
        // Try without path first - connect to root
        format!("wss://{}", CHAINSTACK_WS_HOST)
    } else if endpoint.is_empty() || endpoint == "default" {
        format!("wss://{}", CHAINSTACK_WS_HOST)
    } else {
        endpoint.to_string()
    };

    println!("Connecting to WebSocket at: {} (auth: {})", ws_url, use_auth);

    // Create request with authentication headers
    let mut request = ws_url.into_client_request()
        .map_err(|e| StreamError::ConnectionError(format!("Invalid WebSocket URL: {}", e)))?;

    if use_auth {
        // Add x-token header
        request.headers_mut().insert("x-token", token.parse().unwrap());

        // Add basic auth header
        let auth_value = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_value.as_bytes());
        request.headers_mut().insert("authorization", format!("Basic {}", encoded).parse().unwrap());
    }

    let (ws_stream, _) = connect_async(request)
        .await
        .map_err(|e| StreamError::ConnectionError(format!("WebSocket connection failed: {}", e)))?;

    println!("WebSocket connected");

    // Send subscription request for slots
    let subscribe_request = WsRequest::SlotSubscribe;
    let request_json = serde_json::to_string(&subscribe_request)
        .map_err(|e| StreamError::SubscriptionError(format!("Failed to serialize request: {}", e)))?;

    println!("Sending subscription request: {}", request_json);

    // Split the stream into write and read parts
    let (mut write, mut read) = ws_stream.split();

    // Send the subscription request
    use futures::sink::SinkExt;
    tokio::spawn(async move {
        let _ = write.send(Message::Text(request_json)).await;
    });

    Ok(async_stream::stream! {
        loop {
            match read.next().await {
                Some(Ok(message)) => {
                    match message {
                        Message::Text(text) => {
                            // Parse the WebSocket message
                            if let Ok(ws_response) = serde_json::from_str::<WsResponse>(&text) {
                                match ws_response {
                                    WsResponse::SlotNotification { params } => {
                                        println!("slot: {}", params.result.slot);
                                        yield Ok(RawWebSocketMessage {
                                            slot: params.result.slot,
                                            is_transaction: false,
                                        });
                                    }
                                    WsResponse::TransactionNotification { params } => {
                                        println!("Transaction in slot: {}", params.result.slot);
                                        yield Ok(RawWebSocketMessage {
                                            slot: params.result.slot,
                                            is_transaction: true,
                                        });
                                    }
                                    WsResponse::Error { params } => {
                                        eprintln!("WebSocket error: {}", params.error_message);
                                        yield Err(StreamError::StreamError(params.error_message));
                                    }
                                    WsResponse::AccountNotification { .. } => {
                                        // Ignore account notifications for now
                                    }
                                }
                            } else {
                                // Try to parse as a generic JSON value for debugging
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                    println!("Received message: {}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| text.clone()));
                                }
                            }
                        }
                        Message::Ping(data) => {
                            // Respond to ping with pong
                            yield Err(StreamError::StreamError(format!("Received ping: {:?}", data)));
                        }
                        Message::Pong(_) => {
                            // Ignore pong
                        }
                        Message::Close(_) => {
                            yield Err(StreamError::StreamError("WebSocket closed".to_string()));
                            break;
                        }
                        _ => {}
                    }
                }
                Some(Err(e)) => {
                    yield Err(StreamError::StreamError(format!("WebSocket error: {}", e)));
                    break;
                }
                None => {
                    yield Err(StreamError::StreamError("WebSocket stream ended".to_string()));
                    break;
                }
            }
        }
    })
}
