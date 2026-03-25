use crate::types::Pubkey;
use crate::events::TelemetryEvent;
use crate::parser::WalletDeltas;

/// Minimum SOL delta to consider as a trade (filters out fees)
const MIN_SOL_DELTA: u64 = 1_000_000; // 0.001 SOL in lamports

/// Detect swap events from wallet balance deltas
/// (Legacy interface — active pipeline uses balance_deltas parser + enrichment layer)
pub fn detect_swaps(
    wallet_deltas: &WalletDeltas,
    tracked_mint: &Pubkey,
    _slot: u64,
    _timestamp: i64,
    _tx_signature: String,
) -> Vec<TelemetryEvent> {
    let events = Vec::new();

    let sol_delta = wallet_deltas.effective_sol_delta();
    let _token_delta = wallet_deltas.get_token_delta(tracked_mint);

    if sol_delta.abs() < MIN_SOL_DELTA as i64 {
        return events;
    }

    // Trade detection now happens in enrich_transaction() via the balance delta parser.
    // This module is retained for future use / alternative detection strategies.
    events
}
