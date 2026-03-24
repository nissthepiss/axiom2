use crate::events::trade_event::TradeEvent;
use crate::types::Pubkey;

/// Normalize and validate trade events
///
/// This module ensures trade events meet quality standards before
/// being published to the event bus.
pub struct TradeNormalizer;

impl TradeNormalizer {
    /// Validate a trade event
    pub fn validate(event: &TradeEvent) -> bool {
        // Must have positive amounts
        if event.sol_amount <= 0.0 || event.token_amount <= 0.0 {
            return false;
        }

        // Price must be positive
        if event.price <= 0.0 {
            return false;
        }

        // Wallet must not be default
        if event.wallet == Pubkey::default() {
            return false;
        }

        true
    }

    /// Normalize a trade event (fix common issues)
    pub fn normalize(mut event: TradeEvent) -> Option<TradeEvent> {
        if !Self::validate(&event) {
            return None;
        }

        // Round amounts to reasonable precision
        event.sol_amount = (event.sol_amount * 1_000_000.0).round() / 1_000_000.0;
        event.token_amount = (event.token_amount * 1_000_000.0).round() / 1_000_000.0;
        event.price = (event.price * 1_000_000.0).round() / 1_000_000.0;

        Some(event)
    }

    /// Process a batch of trade events
    pub fn normalize_batch(events: Vec<TradeEvent>) -> Vec<TradeEvent> {
        events
            .into_iter()
            .filter_map(Self::normalize)
            .collect()
    }
}
