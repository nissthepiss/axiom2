#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

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
use crate::events::{EventBus, TelemetryEvent};
use crate::stream::TransactionUpdate;
use crate::types::Pubkey;
use crate::ui::App;
use crate::parser::calculate_balance_deltas as parse_balance_deltas;
use tracing::info;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use futures::{stream::Stream, StreamExt};
use std::io::stdout;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Install default CryptoProvider for rustls
    rustls::crypto::ring::default_provider().install_default().unwrap();

    // Initialize logging to file (not stdout — ratatui owns stdout)
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(std::io::stderr)
        .init();

    // Get mint from user input BEFORE entering raw mode
    let mint_str = resolve_mint()?;
    let mint: Pubkey = mint_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid mint address: {}", e))?;

    // Default RPC endpoint (use public RPC for metadata fetching)
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let endpoint = "chainstack";

    // Fetch token metadata from RPC (before entering TUI)
    eprintln!("Fetching token metadata...");
    let metadata = cli::fetch_token_metadata(&mint, rpc_url).await?;

    // Initialize app state (TUI consumer — receives events, doesn't own engines)
    let mut app = App::new(
        metadata.symbol.clone(),
        metadata.name.clone(),
        mint.to_string(),
    );

    // Fetch real SOL price
    eprintln!("Fetching SOL price...");
    let mut current_sol_price: f64 = 140.0;
    match crate::ui::console::fetch_sol_price().await {
        Some(price) => {
            eprintln!("SOL price: ${:.2}", price);
            current_sol_price = price;
            app.sol_price = price;
        }
        None => {
            eprintln!("WARNING: Failed to fetch SOL price, using fallback ${:.0}", current_sol_price);
        }
    }

    // Initialize engines (ingestion layer — owned by the enrichment pipeline, NOT the TUI)
    let event_bus = EventBus::new(1000);
    let mut fdv_engine = FdvEngine::new();
    let mut price_engine = PriceEngine::new();
    fdv_engine.register_token(mint, metadata.supply, metadata.decimals);

    // Connect to Yellowstone stream
    eprintln!("Connecting to Yellowstone gRPC...");
    let grpc_stream = subscribe_transactions_filtered(endpoint, &mint).await?;
    let mut grpc_stream = std::pin::pin!(grpc_stream);
    app.connected = true;

    // Enter ratatui TUI
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // Install Ctrl+C / shutdown signal handler
    let shutdown = tokio::signal::ctrl_c();
    let mut shutdown = std::pin::pin!(shutdown);

    // Main event loop — races against shutdown signal
    let result = tokio::select! {
        res = run_loop(
            &mut terminal,
            &mut app,
            &mut grpc_stream,
            &mint,
            &event_bus,
            &mut price_engine,
            &mut fdv_engine,
            metadata.decimals,
            &mut current_sol_price,
        ) => res,
        _ = &mut shutdown => {
            eprintln!("\nShutdown signal received, cleaning up...");
            Ok(())
        }
    };

    // === CLEANUP: always runs regardless of how we exited ===
    drop(grpc_stream);
    app.connected = false;

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    if let Err(e) = &result {
        eprintln!("Error: {}", e);
    }

    eprintln!("{} transactions processed", app.tx_count);
    Ok(())
}

// ── Event loop ─────────────────────────────────────────────────
//
// Architecture:
//   gRPC stream → enrich_transaction() → EventBus → App::apply_event()
//
// The TUI only reads from the EventBus. Engines live on the ingestion side.
// Future consumers (Strategy, Logger) subscribe to the same EventBus.

async fn run_loop<S>(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    stream: &mut std::pin::Pin<&mut S>,
    tracked_mint: &Pubkey,
    event_bus: &EventBus,
    price_engine: &mut PriceEngine,
    fdv_engine: &mut FdvEngine,
    decimals: u8,
    current_sol_price: &mut f64,
) -> anyhow::Result<()>
where
    S: Stream<Item = Result<stream::SubscribeUpdate, stream::StreamError>>,
{
    // TUI subscribes to the event bus
    let mut event_rx = event_bus.subscribe();

    // Initial draw
    terminal.draw(|f| ui::draw::draw(f, app))?;

    let mut last_draw = std::time::Instant::now();
    let mut last_sol_price_fetch = std::time::Instant::now();
    let sol_price_interval = Duration::from_secs(30);

    loop {
        // ── 1. SOL price refresh (publishes through bus) ──
        if last_sol_price_fetch.elapsed() >= sol_price_interval {
            last_sol_price_fetch = std::time::Instant::now();
            if let Some(price) = crate::ui::console::fetch_sol_price().await {
                *current_sol_price = price;
                event_bus.publish(TelemetryEvent::SolPriceUpdate { price });

                // Recalculate FDV with new SOL price
                if let Some(fdv_sol) = fdv_engine.get_fdv_sol(tracked_mint) {
                    let fdv_usd = fdv_sol * price;
                    let epoch = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    event_bus.publish(TelemetryEvent::FdvUpdate { epoch_secs: epoch, fdv_usd });
                }
            }
        }

        // ── 2. Keyboard events (local to TUI, no bus) ──
        let has_terminal_event = tokio::task::block_in_place(|| {
            event::poll(Duration::from_millis(5)).unwrap_or(false)
        });

        if has_terminal_event {
            let evt = tokio::task::block_in_place(|| event::read())?;
            if let Event::Key(key) = evt {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Char('t') => {
                            app.chart_timeframe = app.chart_timeframe.next();
                        }
                        KeyCode::F(2) => {
                            app.view_mode = match app.view_mode {
                                crate::ui::console::ViewMode::Normal => crate::ui::console::ViewMode::Alert,
                                crate::ui::console::ViewMode::Alert => crate::ui::console::ViewMode::Normal,
                            };
                        }
                        _ => {}
                    }
                }
            }
        }

        // ── 3. Drain gRPC stream → enrich → publish to EventBus ──
        loop {
            tokio::select! {
                biased;
                Some(result) = futures::StreamExt::next(stream) => {
                    if let Ok(update) = result {
                        if let Some(tx_update) = TransactionUpdate::from_update(&update) {
                            let events = enrich_transaction(
                                tx_update,
                                tracked_mint,
                                price_engine,
                                fdv_engine,
                                decimals,
                                *current_sol_price,
                            );
                            for ev in events {
                                event_bus.publish(ev);
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(1)) => {
                    break;
                }
            }
        }

        // ── 4. Drain EventBus → App::apply_event() ──
        loop {
            match event_rx.try_recv() {
                Ok(event) => {
                    app.apply_event(&event);
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    eprintln!("[warn] event bus lagged, dropped {} events", n);
                }
                Err(_) => break,
            }
        }

        // ── 5. Redraw at fixed interval (200ms) ──
        // Never redraw per-event — batch all updates within the interval
        if last_draw.elapsed() >= Duration::from_millis(200) {
            terminal.draw(|f| ui::draw::draw(f, app))?;
            last_draw = std::time::Instant::now();
        }
    }
}

// ── Enrichment layer ───────────────────────────────────────────
//
// Takes a raw TransactionUpdate, runs it through the parser and engines,
// and returns a Vec of TelemetryEvents ready for consumers.
// Engines (PriceEngine, FdvEngine) live here — they are NOT owned by the TUI.

fn enrich_transaction(
    tx_update: TransactionUpdate,
    tracked_mint: &Pubkey,
    price_engine: &mut PriceEngine,
    fdv_engine: &mut FdvEngine,
    decimals: u8,
    sol_price: f64,
) -> Vec<TelemetryEvent> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut events = Vec::new();

    // Always emit TxProcessed
    events.push(TelemetryEvent::TxProcessed);

    let tx = match tx_update.transaction {
        Some(tx) => tx,
        None => return events,
    };

    let meta_bytes = match &tx.meta_bytes {
        Some(bytes) => bytes,
        None => return events,
    };

    // Extract fee
    if let Ok(meta) = crate::parser::transaction_parser::parse_transaction_meta(meta_bytes) {
        let fee_sol = meta.fee as f64 / 1_000_000_000.0;
        events.push(TelemetryEvent::FeeAccrued { fee_sol });
    }

    // Parse balance deltas into trades
    let trades = match parse_balance_deltas(&tx.message, meta_bytes, tracked_mint) {
        Ok(trades) => trades,
        Err(_) => return events,
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let secs = now % 86400;
    let timestamp = format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);

    for trade in trades {
        let token_amount_abs = trade.token_delta.abs() as f64 / 10_f64.powi(decimals as i32);
        let sol_amount_abs = trade.sol_delta.abs() as f64 / 1_000_000_000.0;

        let price = if token_amount_abs > 0.0 {
            sol_amount_abs / token_amount_abs
        } else {
            0.0
        };

        // Update engines
        price_engine.update_price_for_mint(*tracked_mint, price);
        fdv_engine.update_price(tracked_mint, price);

        // Compute FDV
        let fdv_usd = fdv_engine.get_fdv_sol(tracked_mint).map(|fdv_sol| fdv_sol * sol_price);

        if let Some(fdv) = fdv_usd {
            events.push(TelemetryEvent::FdvUpdate {
                epoch_secs: now,
                fdv_usd: fdv,
            });
        }

        // Format wallet
        let wallet_str = trade.wallet.to_string();
        let wallet_short = format!(
            "{}...{}",
            &wallet_str[..4.min(wallet_str.len())],
            &wallet_str[wallet_str.len().saturating_sub(4)..]
        );

        events.push(TelemetryEvent::Trade {
            timestamp: timestamp.clone(),
            epoch_secs: now,
            is_buy: trade.is_buy,
            sol_amount: sol_amount_abs,
            token_amount: token_amount_abs,
            price,
            wallet: trade.wallet,
            wallet_short,
            fdv_usd,
        });
    }

    events
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

    // IMPORTANT ARCHITECTURE RULE:
    // This system relies exclusively on Yellowstone gRPC transaction streams.
    // Do NOT implement RPC polling, slot-only subscriptions, or websocket fallbacks.

    // Build account_include list: always include the token mint,
    // and for pump.fun tokens also include the bonding curve PDA
    let mut account_include = vec![token_mint.to_string()];

    let mint_str = token_mint.to_string();
    if mint_str.ends_with("pump") {
        // pump.fun program ID
        let pump_program: Pubkey = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
            .parse()
            .expect("valid pump.fun program ID");
        let bonding_curve = Pubkey::derive_pda(
            &[b"bonding-curve", token_mint.as_ref()],
            &pump_program,
        );
        eprintln!("pump.fun token detected — bonding curve: {}", bonding_curve);
        account_include.push(bonding_curve.to_string());
    }

    let mut transactions_map = HashMap::new();
    transactions_map.insert(
        "client".to_string(),
        SubscribeRequestFilterTransactions {
            vote: Some(false),
            failed: Some(false),
            account_include,
            account_exclude: vec![],
            account_required: vec![],
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

    // Create a channel for bidirectional streaming
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    tx.send(subscription).await?;

    // Create request stream from channel receiver
    let request_stream = ReceiverStream::new(rx);

    // Start streaming
    let grpc_stream = client
        .subscribe(request_stream)
        .await?
        .into_inner();

    Ok(grpc_stream.map(|result| result.map_err(|e| stream::StreamError::StreamError(e.to_string()))))
}
