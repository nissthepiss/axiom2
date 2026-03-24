// Generated protobuf types
pub mod geyser {
    tonic::include_proto!("geyser");
}

// Import the generated types at the correct level
use geyser::{
    geyser_client::GeyserClient as TonicGeyserClient,
    SubscribeRequest,
    SubscribeUpdate,
    subscribe_update::UpdateOneof as SubscribeUpdateMessage,
    CommitmentLevel,
    SubscribeRequestFilterSlots,
};
use futures::{stream::Stream, StreamExt};
use std::collections::HashMap;
use std::error::Error as StdError;
use thiserror::Error;
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::metadata::{MetadataKey, MetadataValue};
use base64::Engine as _;
use tokio_stream::wrappers::ReceiverStream;

/// PublicNode - Free public Yellowstone gRPC endpoint (WORKS ON WINDOWS)
const PUBLICNODE_GRPC_HOST: &str = "solana-yellowstone-grpc.publicnode.com";
const PUBLICNODE_GRPC_PORT: u16 = 443;

/// Chainstack Yellowstone gRPC endpoint (returns UNIMPLEMENTED for Subscribe)
const CHAINSTACK_GRPC_HOST: &str = "yellowstone-solana-mainnet.core.chainstack.com";
const CHAINSTACK_GRPC_PORT: u16 = 443;

/// GetBlock Yellowstone gRPC endpoint (requires dedicated node + token)
const GETBLOCK_GRPC_HOST: &str = "go.getblock.io";
const GETBLOCK_GRPC_PORT: u16 = 443;

/// Tatum Yellowstone gRPC endpoint (free API key available)
const TATUM_GRPC_HOST: &str = "solana-mainnet-grpc.gateway.tatum.io";
const TATUM_GRPC_PORT: u16 = 443;

/// Shyft Yellowstone gRPC endpoint
const SHYFT_GRPC_HOST: &str = "grpc.us.shyft.to";
const SHYFT_GRPC_PORT: u16 = 443;

/// Authentication configuration for different providers
pub struct AuthConfig {
    pub token: String,
    pub header_name: String,
    pub basic_auth: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            token: String::new(),
            header_name: "x-token".to_string(),
            basic_auth: None,
        }
    }
}

/// Load credentials from environment variables based on provider
pub fn load_auth_config(endpoint: &str) -> AuthConfig {
    if endpoint.contains("publicnode") {
        // PublicNode/AllNodes uses x-token header
        let token = std::env::var("PUBLICNODE_TOKEN")
            .or_else(|_| std::env::var("ALLNODES_TOKEN"))
            .unwrap_or_default();

        AuthConfig {
            token,
            header_name: "x-token".to_string(),
            basic_auth: None,
        }
    } else if endpoint.contains("chainstack") {
        // Chainstack uses x-token + basic auth
        let token = std::env::var("CHAINSTACK_X_TOKEN").unwrap_or_default();
        let username = std::env::var("CHAINSTACK_USERNAME").unwrap_or_default();
        let password = std::env::var("CHAINSTACK_PASSWORD").unwrap_or_default();

        let basic_auth = if !username.is_empty() && !password.is_empty() {
            let auth_value = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(auth_value);
            Some(format!("Basic {}", encoded))
        } else {
            None
        };

        AuthConfig {
            token,
            header_name: "x-token".to_string(),
            basic_auth,
        }
    } else if endpoint.contains("getblock") {
        // GetBlock uses token in URL path, but we can also use x-token
        let token = std::env::var("GETBLOCK_TOKEN").unwrap_or_default();

        AuthConfig {
            token,
            header_name: "x-token".to_string(),
            basic_auth: None,
        }
    } else if endpoint.contains("tatum") {
        // Tatum uses x-api-key header
        let token = std::env::var("TATUM_API_KEY")
            .or_else(|_| std::env::var("API_KEY"))
            .unwrap_or_default();

        AuthConfig {
            token,
            header_name: "x-api-key".to_string(),
            basic_auth: None,
        }
    } else if endpoint.contains("shyft") {
        // Shyft uses x-token header
        let token = std::env::var("SHYFT_TOKEN").unwrap_or_default();

        AuthConfig {
            token,
            header_name: "x-token".to_string(),
            basic_auth: None,
        }
    } else {
        // Default: try generic TOKEN variable
        let token = std::env::var("TOKEN")
            .or_else(|_| std::env::var("API_KEY"))
            .unwrap_or_default();

        AuthConfig {
            token,
            header_name: "x-token".to_string(),
            basic_auth: None,
        }
    }
}

#[derive(Error, Debug)]
pub enum StreamError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Subscription failed: {0}")]
    SubscriptionError(String),
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
    // Load authentication configuration based on endpoint
    let auth_config = load_auth_config(endpoint);

    // Determine the actual endpoint to use
    let actual_endpoint = if endpoint.contains("publicnode") {
        // PublicNode - Free public Yellowstone endpoint (WORKS ON WINDOWS)
        format!("https://{}:{}", PUBLICNODE_GRPC_HOST, PUBLICNODE_GRPC_PORT)
    } else if endpoint.contains("chainstack") || endpoint.contains("yellowstone-solana-mainnet") {
        format!("https://{}:{}", CHAINSTACK_GRPC_HOST, CHAINSTACK_GRPC_PORT)
    } else if endpoint.contains("getblock") {
        // GetBlock Yellowstone gRPC (requires token)
        format!("https://{}:{}", GETBLOCK_GRPC_HOST, GETBLOCK_GRPC_PORT)
    } else if endpoint.contains("tatum") {
        // Tatum - Free API key available at tatum.io/chain/solana
        format!("https://{}:{}", TATUM_GRPC_HOST, TATUM_GRPC_PORT)
    } else if endpoint.contains("shyft") {
        // Shyft - Yellowstone gRPC endpoint
        format!("https://{}:{}", SHYFT_GRPC_HOST, SHYFT_GRPC_PORT)
    } else if endpoint.contains("genesysgo") {
        "https://yellowstone.genesysgo.net:443".to_string()
    } else if endpoint.is_empty() || endpoint == "default" {
        // Use Tatum by default (free API key available)
        format!("https://{}:{}", TATUM_GRPC_HOST, TATUM_GRPC_PORT)
    } else {
        endpoint.to_string()
    };

    let use_auth = !auth_config.token.is_empty() ||
                    actual_endpoint.contains("chainstack") ||
                    actual_endpoint.contains(CHAINSTACK_GRPC_HOST) ||
                    actual_endpoint.contains(TATUM_GRPC_HOST) ||
                    actual_endpoint.contains(SHYFT_GRPC_HOST);

    println!("Connecting to Yellowstone at: {} (auth: {})", actual_endpoint, use_auth);
    if use_auth {
        println!("Auth header: {}", auth_config.header_name);
        if !auth_config.token.is_empty() {
            println!("Token: {}***", &auth_config.token[..auth_config.token.len().min(8)]);
        } else {
            println!("WARNING: No token configured - set environment variable");
        }
    }
    println!("Service: yellowstone.grpc.Geyser");
    println!("Method: Subscribe");

    // Check if we need TLS - for all HTTPS endpoints
    let use_tls = actual_endpoint.starts_with("https://");

    // Create channel with TLS configuration
    let channel = Channel::from_shared(actual_endpoint)
        .map_err(|e| StreamError::ConnectionError(format!("Failed to create channel: {}", e)))?
        .timeout(std::time::Duration::from_secs(30));

    // Add TLS config for HTTPS endpoints
    let channel = if use_tls {
        // For development/testing, use native roots
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

    // Create client with authentication interceptor
    let header_name = auth_config.header_name.clone();
    let token_value = auth_config.token.clone();
    let basic_auth = auth_config.basic_auth.clone();

    println!("Using authentication: {}={}, basic_auth={}",
             header_name,
             !token_value.is_empty(),
             basic_auth.is_some());

    // Clone use_auth for the closure
    let use_auth_copy = use_auth;

    let mut client = TonicGeyserClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        // Add authentication header
        if use_auth_copy {
            // Add the token header (x-token, x-api-key, etc.)
            if !token_value.is_empty() {
                match MetadataKey::from_bytes(header_name.as_bytes()) {
                    Ok(key) => {
                        match MetadataValue::try_from(&token_value) {
                            Ok(val) => {
                                req.metadata_mut().insert(key, val);
                            }
                            Err(_) => return Err(tonic::Status::unauthenticated("Invalid token format")),
                        }
                    }
                    Err(_) => return Err(tonic::Status::unauthenticated("Invalid header name")),
                }
            }

            // Add basic authorization header (for Chainstack)
            if let Some(auth_value) = &basic_auth {
                match MetadataValue::try_from(auth_value.as_str()) {
                    Ok(val) => {
                        req.metadata_mut().insert("authorization", val);
                    }
                    Err(_) => {
                        // If basic auth fails, still try with just the token
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
            filter_by_commitment: Some(false),
        },
    );

    let subscription = SubscribeRequest {
        accounts: HashMap::new(),
        slots: slots_map,
        transactions: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
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
        match &update.update_oneof {
            Some(SubscribeUpdateMessage::Transaction(tx)) => {
                println!("✓ Transaction in slot: {}", tx.slot);
                if let Some(transaction) = &tx.transaction {
                    println!("  Signature: {}", bs58::encode(&transaction.signature).into_string());
                }

                Some(TransactionUpdate {
                    slot: tx.slot,
                    transaction: Some(OurTransaction {
                        signatures: vec![tx.transaction.as_ref().map(|t| t.signature.clone()).unwrap_or_default()],
                        message: tx.transaction.as_ref().map(|t| t.transaction.clone()).unwrap_or_default(),
                        meta: None,
                    }),
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
            Some(SubscribeUpdateMessage::Pong(_pong)) => {
                println!("pong received");
                None
            }
            Some(SubscribeUpdateMessage::BlockMeta(block_meta)) => {
                println!("block meta: {}", block_meta.slot);
                None
            }
            _ => None,
        }
    }
}
