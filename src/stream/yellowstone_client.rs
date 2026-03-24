// Generated protobuf types
pub mod geyser {
    tonic::include_proto!("yellowstone.grpc");
}

// Import the generated types at the correct level
use geyser::{
    geyser_client::GeyserClient as TonicGeyserClient,
    SubscribeRequest,
    SubscribeUpdate,
    subscribe_request::{Message as SubscribeRequestMessage, SubscribeRequestSubscribe, SubscribeRequestFilterTransactions, SubscribeRequestFilterSlots, FailWithConfig},
    subscribe_update::Message as SubscribeUpdateMessage,
    CommitmentLevel,
};
use futures::{stream::Stream, StreamExt};
use std::collections::HashMap;
use std::error::Error as StdError;
use thiserror::Error;
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::metadata::{MetadataKey, MetadataValue};
use base64::Engine as _;
use tokio_stream::wrappers::ReceiverStream;

/// Chainstack Yellowstone gRPC endpoint
const CHAINSTACK_GRPC_HOST: &str = "yellowstone-solana-mainnet.core.chainstack.com";
const CHAINSTACK_GRPC_PORT: u16 = 443;

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

    #[error("RPC error: {0}")]
    RpcError(String),
}

impl From<anyhow::Error> for StreamError {
    fn from(e: anyhow::Error) -> Self {
        StreamError::ConnectionError(e.to_string())
    }
}

/// Transaction data extracted from gRPC update
#[derive(Debug, Clone)]
pub struct TransactionData {
    pub slot: u64,
    pub signatures: Vec<Vec<u8>>,
    pub message: Vec<u8>,
    pub meta: Option<TransactionMeta>,
}

/// Transaction metadata
#[derive(Debug, Clone)]
pub struct TransactionMeta {
    pub error: Option<Vec<u8>>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
}

/// Token balance data
#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_index: u32,
    pub mint: Vec<u8>,
    pub ui_token_amount: UiTokenAmount,
    pub owner: Option<Vec<u8>>,
}

/// UI token amount
#[derive(Debug, Clone)]
pub struct UiTokenAmount {
    pub amount: String,
    pub decimals: u32,
    pub ui_amount_string: String,
    pub ui_amount: f64,
}

/// Our simplified transaction update type
pub struct TransactionUpdate {
    pub slot: u64,
    pub transaction: Option<OurTransaction>,
}

/// Our transaction structure compatible with existing parser
#[derive(Debug, Clone)]
pub struct OurTransaction {
    pub signatures: Vec<Vec<u8>>,
    pub message: Vec<u8>,
    pub meta: Option<TransactionMeta>,
}

/// Subscribe to Yellowstone gRPC transaction stream
pub async fn subscribe_transactions(
    endpoint: &str,
) -> Result<impl Stream<Item = Result<SubscribeUpdate, StreamError>>, StreamError> {
    // Load credentials from environment
    let (token, username, password) = load_chainstack_credentials();

    // Determine the actual endpoint to use
    let actual_endpoint = if endpoint.contains("chainstack") || endpoint.contains("yellowstone-solana-mainnet") {
        format!("https://{}:{}", CHAINSTACK_GRPC_HOST, CHAINSTACK_GRPC_PORT)
    } else if endpoint.contains("genesysgo") {
        "https://yellowstone.genesysgo.net:443".to_string()
    } else if endpoint.contains("alchemy") {
        "https://solana-mainnet.g.alchemy.com:443".to_string()
    } else if endpoint.is_empty() || endpoint == "default" {
        // Use Chainstack by default
        format!("https://{}:{}", CHAINSTACK_GRPC_HOST, CHAINSTACK_GRPC_PORT)
    } else {
        endpoint.to_string()
    };

    let use_auth = actual_endpoint.contains("chainstack") || actual_endpoint.contains(CHAINSTACK_GRPC_HOST);

    println!("Connecting to Yellowstone at: {} (auth: {})", actual_endpoint, use_auth);
    println!("Service: yellowstone.grpc.Geyser");
    println!("Method: Subscribe");

    // Check if we need TLS - for Chainstack, always use TLS
    let use_tls = use_auth || actual_endpoint.starts_with("https://");

    // Create channel with TLS configuration
    let channel = Channel::from_shared(actual_endpoint)
        .map_err(|e| StreamError::ConnectionError(format!("Failed to create channel: {}", e)))?
        .timeout(std::time::Duration::from_secs(30));

    // Add TLS config for HTTPS endpoints
    let channel = if use_tls {
        // For development, skip certificate verification
        // TODO: Enable proper certificate verification for production
        channel.tls_config(
            ClientTlsConfig::new().with_native_roots()
        )
        .map_err(|e| StreamError::ConnectionError(format!("Failed to configure TLS: {}", e)))?
    } else {
        channel
    };

    let channel = channel.connect()
        .await
        .map_err(|e| {
            let err_msg = format!("Failed to connect: {}", e);
            eprintln!("Connection error details: {:?}", e);
            if let Some(source) = e.source() {
                eprintln!("Error source: {}", source);
            }
            StreamError::ConnectionError(err_msg)
        })?;

    println!("Yellowstone client created");

    // Create client with authentication (for Chainstack)
    // Always use interceptor for consistent type
    let token_header = token.clone();
    let basic_auth = if use_auth {
        let auth_value = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(auth_value);
        Some(format!("Basic {}", encoded))
    } else {
        None
    };

    println!("Using authentication: x-token={}, basic_auth={}", !token_header.is_empty(), basic_auth.is_some());

    // Clone use_auth for the closure
    let use_auth_copy = use_auth;

    let mut client = TonicGeyserClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        // Add x-token header (for Chainstack)
        if use_auth_copy {
            match MetadataValue::try_from(&token_header) {
                Ok(val) => {
                    req.metadata_mut().insert("x-token", val);
                }
                Err(_) => return Err(tonic::Status::unauthenticated("Invalid token format")),
            }

            // Add basic authorization header
            if let Some(auth_value) = &basic_auth {
                match MetadataValue::try_from(auth_value.as_str()) {
                    Ok(val) => {
                        req.metadata_mut().insert("authorization", val);
                    }
                    Err(_) => {
                        // If basic auth fails, still try with just x-token
                    }
                }
            }
        }

        Ok(req)
    });

    // Create subscription request - subscribe only to slots with filter
    let mut slots_map = HashMap::new();
    slots_map.insert(
        "client".to_string(),
        SubscribeRequestFilterSlots {
            filter_by_commitment: false,
        },
    );

    let subscription = SubscribeRequest {
        message: Some(SubscribeRequestMessage::Subscribe(
            SubscribeRequestSubscribe {
                accounts: HashMap::new(),
                slots: slots_map,
                transactions: HashMap::new(),
                blocks: HashMap::new(),
                blocks_meta: HashMap::new(),
                entry: HashMap::new(),
                commitment: CommitmentLevel::CommitmentProcessed as i32,
                account_data_slice: vec![],
                ping: HashMap::new(),
            },
        )),
    };

    println!("Subscribing to slots...");

    // Create a channel for bidirectional streaming
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    // Send the initial subscription request
    tx.send(subscription).await
        .map_err(|e| StreamError::SubscriptionError(format!("Failed to send subscription request: {}", e)))?;

    // Create request stream from channel receiver
    let request_stream = ReceiverStream::new(rx);

    // Start streaming with proper bidirectional request stream
    let stream = client
        .subscribe(request_stream)
        .await
        .map_err(|e: tonic::Status| StreamError::SubscriptionError(format!("Subscription failed: {}", e)))?
        .into_inner();

    Ok(stream.map(|result: Result<SubscribeUpdate, tonic::Status>| {
        result.map_err(|e| StreamError::StreamError(e.to_string()))
    }))
}

impl TransactionUpdate {
    pub fn from_update(update: &SubscribeUpdate) -> Option<Self> {
        match &update.message {
            Some(SubscribeUpdateMessage::Transaction(tx)) => {
                println!("✓ Transaction in slot: {}", tx.slot);
                if let Some(transaction) = &tx.transaction {
                    let empty_sig = vec![];
                    let sig = transaction.signatures.get(0).unwrap_or(&empty_sig);
                    println!("  Signature: {}", bs58::encode(sig).into_string());
                }

                Some(TransactionUpdate {
                    slot: tx.slot,
                    transaction: Some(OurTransaction {
                        signatures: tx.transaction.as_ref()
                            .map(|t| t.signatures.clone())
                            .unwrap_or_default(),
                        message: tx.transaction.as_ref()
                            .map(|t| t.message.clone())
                            .unwrap_or_default(),
                        meta: tx.transaction.as_ref()
                            .and_then(|t| t.meta.as_ref())
                            .map(|m| TransactionMeta {
                                error: if m.error.is_empty() { None } else { Some(m.error.clone()) },
                                fee: m.fee,
                                pre_balances: m.pre_balances.clone(),
                                post_balances: m.post_balances.clone(),
                                pre_token_balances: vec![],
                                post_token_balances: vec![],
                            }),
                    }),
                })
            }
            Some(SubscribeUpdateMessage::TransactionStatus(status)) => {
                Some(TransactionUpdate {
                    slot: status.slot,
                    transaction: None,
                })
            }
            Some(SubscribeUpdateMessage::Slot(slot)) => {
                println!("slot: {}", slot.slot);
                None
            }
            Some(SubscribeUpdateMessage::Ping(_ping)) => {
                println!("ping received");
                None
            }
            _ => None,
        }
    }
}
