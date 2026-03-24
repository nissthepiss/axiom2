mod cli;
mod engine;
mod events;
mod parser;
mod stream;
mod swap;
mod types;
mod ui;

use cli::resolve_mint;
use crate::engine::{FdvEngine, PriceEngine};
use crate::events::EventBus;
use crate::stream::{OurTransaction, TransactionUpdate};
use crate::types::Pubkey;
use crate::ui::ConsoleRenderer;
use tracing::info;

use futures::{stream::Stream, StreamExt};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install default CryptoProvider for rustls
    rustls::crypto::ring::default_provider().install_default().unwrap();

    // Initialize logging (minimal output for clean console)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();

    // Get mint from user input (or default)
    let mint_str = resolve_mint()?;
    let mint: Pubkey = mint_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid mint address: {}", e))?;

    // Default RPC endpoint (use public RPC for metadata fetching)
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let endpoint = "chainstack";

    // Fetch token metadata from RPC
    let metadata = cli::fetch_token_metadata(&mint, rpc_url).await?;

    // Print tracking info
    println!("\nTracking {}", metadata.symbol);
    println!("Mint: {}", mint);

    // Initialize engines
    let event_bus = EventBus::new(1000);
    let mut console = ConsoleRenderer::new(metadata.symbol.clone(), metadata.name.clone());
    let mut fdv_engine = FdvEngine::new();
    let mut price_engine = PriceEngine::new();

    // Register token in FDV engine
    fdv_engine.register_token(mint, metadata.supply, metadata.decimals);

    // Print header with initial FDV
    console.print_banner();
    console.print_fdv_header(None);

    // Subscribe to Yellowstone stream with transaction filter
    info!("Connecting to Yellowstone gRPC at {}...", endpoint);
    let stream = subscribe_transactions_filtered(endpoint, &mint).await?;
    let mut stream = std::pin::pin!(stream);

    info!("Streaming transactions...");

    // Main event loop with latency tracking
    loop {
        tokio::select! {
            // Process incoming stream updates
            Some(result) = async {
                let start = Instant::now();
                let item = futures::StreamExt::next(&mut stream).await;
                let latency = start.elapsed();
                item.map(|r| (r, latency))
            } => {
                let (update_result, latency) = result;

                if let Ok(update) = update_result {
                    if let Some(tx_update) = TransactionUpdate::from_update(&update) {
                        process_transaction(
                            tx_update,
                            &mint,
                            &event_bus,
                            &mut price_engine,
                            &mut fdv_engine,
                            latency,
                            metadata.decimals,
                            &mut console,
                        ).await;
                    }
                }
            }

            // Handle shutdown
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
        }
    }

    Ok(())
}

/// Subscribe to Yellowstone gRPC with transaction filter for specific token
async fn subscribe_transactions_filtered(
    endpoint: &str,
    token_mint: &Pubkey,
) -> anyhow::Result<impl Stream<Item = Result<stream::SubscribeUpdate, stream::StreamError>>> {
    use stream::geyser::{
        geyser_client::GeyserClient as TonicGeyserClient,
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions,
    };
    use std::collections::HashMap;
    use stream::load_auth_config;
    use tonic::metadata::MetadataKey;
    use tonic::transport::{Channel, ClientTlsConfig};
    use tokio_stream::wrappers::ReceiverStream;

    // Load authentication configuration
    let auth_config = load_auth_config(endpoint);

    // Determine the actual endpoint to use
    let actual_endpoint = if endpoint.contains("chainstack") || endpoint.contains("yellowstone-solana-mainnet") {
        "https://yellowstone-solana-mainnet.core.chainstack.com:443".to_string()
    } else if endpoint.contains("publicnode") {
        "https://solana-yellowstone-grpc.publicnode.com:443".to_string()
    } else if endpoint.contains("tatum") {
        "https://solana-mainnet-grpc.gateway.tatum.io:443".to_string()
    } else {
        endpoint.to_string()
    };

    let use_auth = !auth_config.token.is_empty() ||
                    actual_endpoint.contains("chainstack");

    // Create channel with TLS
    let channel = Channel::from_shared(actual_endpoint)?
        .timeout(std::time::Duration::from_secs(30))
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;

    // Create client with authentication interceptor
    let header_name = auth_config.header_name.clone();
    let token_value = auth_config.token.clone();
    let basic_auth = auth_config.basic_auth.clone();

    let mut client = TonicGeyserClient::with_interceptor(channel, move |mut req: tonic::Request<()>| {
        if use_auth && !token_value.is_empty() {
            if let Ok(key) = MetadataKey::from_bytes(header_name.as_bytes()) {
                if let Ok(val) = tonic::metadata::MetadataValue::try_from(&token_value) {
                    req.metadata_mut().insert(key, val);
                }
            }

            // Add basic auth if present
            if let Some(auth_value) = &basic_auth {
                if let Ok(val) = tonic::metadata::MetadataValue::try_from(auth_value.as_str()) {
                    req.metadata_mut().insert("authorization", val);
                }
            }
        }

        Ok(req)
    });

    // Create subscription request filtered by token mint
    let mut transactions_map = HashMap::new();
    transactions_map.insert(
        "client".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include: vec![token_mint.to_string()],
            account_exclude: vec![],
        },
    );

    let subscription = SubscribeRequest {
        accounts: HashMap::new(),
        slots: HashMap::new(),
        transactions: transactions_map,
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        commitment: Some(CommitmentLevel::Processed as i32),
        accounts_data_slice: vec![],
        ping: None,
    };

    println!("Subscribing to transactions for token: {}", token_mint);

    // Create a channel for bidirectional streaming
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tx.send(subscription).await?;

    // Create request stream from channel receiver
    let request_stream = ReceiverStream::new(rx);

    // Start streaming
    let stream = client
        .subscribe(request_stream)
        .await?
        .into_inner();

    Ok(stream.map(|result| result.map_err(|e| stream::StreamError::StreamError(e.to_string()))))
}

/// Process a transaction update with balance delta calculation
async fn process_transaction(
    tx_update: TransactionUpdate,
    tracked_mint: &Pubkey,
    _event_bus: &EventBus,
    price_engine: &mut PriceEngine,
    fdv_engine: &mut FdvEngine,
    latency: std::time::Duration,
    decimals: u8,
    console: &mut ConsoleRenderer,
) {

    match tx_update {
        TransactionUpdate {
            slot: _,
            transaction: Some(tx),
            ..
        } => {
            // Calculate balance deltas
            if let Some((token_delta, sol_delta, wallet)) = calculate_balance_deltas(&tx, tracked_mint) {
                // Determine trade type
                let is_buy = token_delta > 0 && sol_delta < 0;
                let is_sell = token_delta < 0 && sol_delta > 0;

                // Filter out tiny SOL movements (fees)
                const MIN_SOL_DELTA: i64 = 1_000_000; // 0.001 SOL in lamports

                if is_buy && sol_delta.abs() < MIN_SOL_DELTA {
                    return; // Ignore fee-only movements
                }

                if is_buy || is_sell {
                    let trade_type = if is_buy { "BUY" } else { "SELL" };

                    // Calculate price
                    let token_amount_abs = token_delta.abs() as f64 / 10_f64.powi(decimals as i32);
                    let sol_amount_abs = sol_delta.abs() as f64 / 1_000_000_000.0; // Convert lamports to SOL

                    let price = if token_amount_abs > 0.0 {
                        sol_amount_abs / token_amount_abs
                    } else {
                        0.0
                    };

                    // Update price engine
                    price_engine.update_price_for_mint(*tracked_mint, price);

                    // Update FDV
                    if let Some(current_fdv) = fdv_engine.get_fdv_usd(tracked_mint, 0.0) {
                        console.print_fdv_header(Some(current_fdv));
                    }

                    // Format wallet address (first 4 + last 4 chars)
                    let wallet_short = format!(
                        "{}...{}",
                        &wallet.to_string()[..4],
                        &wallet.to_string()[wallet.to_string().len()-4..]
                    );

                    // Format timestamp
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let secs = now % 86400;
                    let hours = secs / 3600;
                    let minutes = (secs % 3600) / 60;
                    let timestamp = format!("{:02}:{:02}:{:02}", hours, minutes, secs % 60);

                    // Print trade
                    println!("{} {:>4} {:.2} SOL {} {}ms",
                        timestamp,
                        trade_type,
                        sol_amount_abs,
                        wallet_short,
                        latency.as_millis()
                    );
                }
            }
        }
        _ => {}
    }
}

/// Calculate balance deltas for a transaction
fn calculate_balance_deltas(
    _tx: &OurTransaction,
    _tracked_mint: &Pubkey,
) -> Option<(i64, i64, Pubkey)> {
    // For now, we need to parse the transaction to get balance changes
    // The current proto structure doesn't include pre/post balances
    // TODO: Implement full transaction parsing

    // This is a placeholder - we need to parse the transaction message
    // to extract actual balance changes

    None
}

