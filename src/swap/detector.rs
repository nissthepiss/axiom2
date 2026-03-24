use crate::types::Pubkey;
use tracing::debug;

use crate::events::trade_event::{TradeEvent, TradeSide};
use crate::parser::WalletDeltas;

/// Minimum SOL delta to consider as a trade (filters out fees)
const MIN_SOL_DELTA: u64 = 1_000_000; // 0.001 SOL in lamports

/// Detect swap events from wallet balance deltas
pub fn detect_swaps(
    wallet_deltas: &WalletDeltas,
    tracked_mint: &Pubkey,
    slot: u64,
    timestamp: i64,
    tx_signature: String,
) -> Vec<TradeEvent> {
    let mut events = Vec::new();

    let sol_delta = wallet_deltas.effective_sol_delta();
    let token_delta = wallet_deltas.get_token_delta(tracked_mint);

    // Apply fee filter
    if sol_delta.abs() < MIN_SOL_DELTA as i64 {
        debug!(
            "SOL delta {} below fee threshold, skipping",
            sol_delta as f64 / 1_000_000_000.0
        );
        return events;
    }

    // BUY: SOL decreases, token increases
    if sol_delta < 0 && token_delta > 0 {
        let sol_amount = (sol_delta.abs() as f64) / 1_000_000_000.0; // Convert to SOL
        let token_amount = token_delta as f64;
        let price = sol_amount / token_amount;

        events.push(TradeEvent::new(
            *tracked_mint,
            TradeSide::Buy,
            sol_amount,
            token_amount,
            wallet_deltas.owner,
            price,
            slot,
            timestamp,
            tx_signature,
        ));
    }
    // SELL: SOL increases, token decreases
    else if sol_delta > 0 && token_delta < 0 {
        let sol_amount = sol_delta as f64 / 1_000_000_000.0; // Convert to SOL
        let token_amount = token_delta.abs() as f64;
        let price = sol_amount / token_amount;

        events.push(TradeEvent::new(
            *tracked_mint,
            TradeSide::Sell,
            sol_amount,
            token_amount,
            wallet_deltas.owner,
            price,
            slot,
            timestamp,
            tx_signature,
        ));
    }

    events
}

/// Detect swaps for all wallets in a transaction
pub fn detect_swaps_for_transaction(
    wallet_deltas: &[WalletDeltas],
    tracked_mint: &Pubkey,
    slot: u64,
    timestamp: i64,
    tx_signature: String,
) -> Vec<TradeEvent> {
    let mut all_events = Vec::new();

    for wallet_delta in wallet_deltas {
        let events = detect_swaps(
            wallet_delta,
            tracked_mint,
            slot,
            timestamp,
            tx_signature.clone(),
        );
        all_events.extend(events);
    }

    all_events
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_detect_buy() {
        let mut deltas = WalletDeltas::new(Pubkey::default());
        deltas.sol_delta = -2_000_000_000; // -2 SOL
        deltas.token_deltas.insert(Pubkey::default(), 1_000_000);

        let events = detect_swaps(&deltas, &Pubkey::default(), 100, 0, "sig".to_string());
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].side, TradeSide::Buy));
    }

    #[test]
    fn test_fee_filter() {
        let mut deltas = WalletDeltas::new(Pubkey::default());
        deltas.sol_delta = -500_000; // -0.0005 SOL (below threshold)
        deltas.token_deltas.insert(Pubkey::default(), 1000);

        let events = detect_swaps(&deltas, &Pubkey::default(), 100, 0, "sig".to_string());
        assert_eq!(events.len(), 0);
    }
}
