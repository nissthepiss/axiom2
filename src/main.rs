mod cli;
mod engine;
mod events;
// Temporarily disable metadata module on Windows due to OpenSSL issues
// mod metadata;
mod parser;
mod stream;
mod swap;
mod ui;
mod types;

use clap::Parser;
use engine::{FdvEngine, PriceEngine, SlotEngine};
use events::EventBus;
// use metadata::{fetch_mint_account, fetch_metadata, TokenContext};
use parser::{contains_tracked_mint, BalanceDeltaResult};
use stream::{subscribe_transactions, TransactionUpdate};
use swap::{detect_swaps_for_transaction, TradeNormalizer};
use tracing::{error, info, warn};
use ui::ConsoleRenderer;

// Use our own Pubkey type
use types::Pubkey;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Install default CryptoProvider for rustls
    rustls::crypto::ring::default_provider().install_default().unwrap();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Parse CLI arguments
    let cli = cli::Cli::parse();
    let mint = cli.parse_mint()?;

    info!("Starting Axiom Telemetry");
    info!("Tracking mint: {}", mint);
    info!("Yellowstone endpoint: {}", cli.endpoint);
    info!("RPC endpoint: {}", cli.rpc);

    // For now, use default values instead of fetching metadata
    // This avoids the OpenSSL dependency issue on Windows
    // TODO: Implement metadata fetching without OpenSSL dependency
    let symbol = "TOKEN".to_string();
    let name = "Unknown Token".to_string();
    let supply: u64 = 1_000_000_000;
    let decimals: u8 = 9;

    info!("Token: {} ({})", name, symbol);
    info!("Supply: {} ({} decimals)", supply, decimals);

    // Initialize components
    let event_bus = EventBus::new(1000);
    let mut console = ConsoleRenderer::new(symbol, name);

    // Print banner and initial header
    console.print_banner();
    console.print_header(None);

    // Spawn console renderer task
    let event_bus_console = event_bus.clone();
    let console_task = tokio::spawn(async move {
        let mut receiver = event_bus_console.subscribe();
        while let Ok(trade) = receiver.recv().await {
            console.print_trade(&trade);
        }
    });

    // Spawn FDV engine task
    let event_bus_fdv = event_bus.clone();
    let fdv_task = tokio::spawn(async move {
        let mut receiver = event_bus_fdv.subscribe();
        let mut local_slot_engine = SlotEngine::new();
        let mut local_price_engine = PriceEngine::new();
        let mut local_fdv_engine = FdvEngine::new();
        local_fdv_engine.register_token(mint, supply, decimals);

        while let Ok(trade) = receiver.recv().await {
            // Update price from trade
            local_price_engine.update_price(&trade);

            // Process through slot engine
            if let Some(finalized_price) = local_slot_engine.process_trade(&trade) {
                local_fdv_engine.update_price(&trade.mint, finalized_price);
            }
        }
    });

    // Subscribe to Yellowstone stream
    info!("Connecting to Yellowstone gRPC...");
    let stream = subscribe_transactions(&cli.endpoint).await?;
    let mut stream = std::pin::pin!(stream);

    info!("Streaming transactions...");

    // Main event loop
    loop {
        tokio::select! {
            // Process incoming stream updates
            result = async {
                futures::StreamExt::next(&mut stream).await.ok_or_else(|| anyhow::anyhow!("Stream ended"))?
            } => {
                let update = result?;

                if let Some(tx_update) = TransactionUpdate::from_update(&update) {
                    process_transaction(
                        tx_update,
                        &mint,
                        &event_bus,
                    ).await;
                }
            }

            // Handle shutdown
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}

/// Process a transaction update
async fn process_transaction(
    tx_update: TransactionUpdate,
    tracked_mint: &Pubkey,
    event_bus: &EventBus,
) {
    use stream::OurTransaction;

    match tx_update {
        TransactionUpdate {
            slot,
            transaction: Some(tx),
            ..
        } => {
            // Convert our transaction to a format the parser can handle
            // For now, just log it to verify streaming is working
            info!("Received transaction in slot {}", slot);
            if let Some(sig) = tx.signatures.first() {
                info!("Signature: {}", bs58::encode(sig).into_string());
            }

            // TODO: Parse the transaction properly
            // For now, we're just verifying the stream works
        }
        _ => {}
    }
}
