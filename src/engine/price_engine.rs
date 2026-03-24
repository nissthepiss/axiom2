use crate::events::trade_event::TradeEvent;
use crate::types::Pubkey;
use std::collections::HashMap;

/// Price engine for tracking trade prices
///
/// Maintains the last trade price for each token.
pub struct PriceEngine {
    prices: HashMap<Pubkey, f64>,
}

impl PriceEngine {
    pub fn new() -> Self {
        Self {
            prices: HashMap::new(),
        }
    }

    /// Update price for a token based on a trade
    pub fn update_price(&mut self, trade: &TradeEvent) {
        self.prices.insert(trade.mint, trade.price);
    }

    /// Get current price for a token
    pub fn get_price(&self, mint: &Pubkey) -> Option<f64> {
        self.prices.get(mint).copied()
    }

    /// Set price for a token (e.g., from slot engine)
    pub fn set_price(&mut self, mint: Pubkey, price: f64) {
        self.prices.insert(mint, price);
    }
}

impl Default for PriceEngine {
    fn default() -> Self {
        Self::new()
    }
}
