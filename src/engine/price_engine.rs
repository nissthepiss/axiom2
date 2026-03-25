use crate::types::Pubkey;
use std::collections::HashMap;

/// Price engine for tracking trade prices.
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

    /// Get current price for a token
    pub fn get_price(&self, mint: &Pubkey) -> Option<f64> {
        self.prices.get(mint).copied()
    }

    /// Set price for a token
    pub fn set_price(&mut self, mint: Pubkey, price: f64) {
        self.prices.insert(mint, price);
    }

    /// Update price for a specific mint
    pub fn update_price_for_mint(&mut self, mint: Pubkey, price: f64) {
        self.prices.insert(mint, price);
    }
}

impl Default for PriceEngine {
    fn default() -> Self {
        Self::new()
    }
}
