use crate::types::Pubkey;

/// Core event type flowing through the EventBus.
/// Produced by the ingestion/enrichment layer, consumed by TUI, loggers, strategies.
#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    /// A parsed, enriched trade from a transaction
    Trade {
        timestamp: String,
        epoch_secs: u64,
        is_buy: bool,
        sol_amount: f64,
        token_amount: f64,
        price: f64,
        wallet: Pubkey,
        wallet_short: String,
        fdv_usd: Option<f64>,
    },

    /// FDV/market cap data point
    FdvUpdate {
        epoch_secs: u64,
        fdv_usd: f64,
    },

    /// SOL/USD price refreshed
    SolPriceUpdate {
        price: f64,
    },

    /// Transaction fee accumulated (SOL)
    FeeAccrued {
        fee_sol: f64,
    },

    /// A transaction was processed (increments counter)
    TxProcessed,
}
