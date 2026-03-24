use crate::types::Pubkey;
use std::collections::HashMap;

/// FDV (Fully Diluted Valuation) engine
///
/// Calculates and tracks FDV for tracked tokens.
/// FDV = price * total_supply
pub struct FdvEngine {
    token_supplies: HashMap<Pubkey, u64>, // Supply in token base units
    token_decimals: HashMap<Pubkey, u8>,
    current_prices: HashMap<Pubkey, f64>,  // Price in SOL per token
    current_fdv: HashMap<Pubkey, f64>,     // FDV in SOL
}

impl FdvEngine {
    pub fn new() -> Self {
        Self {
            token_supplies: HashMap::new(),
            token_decimals: HashMap::new(),
            current_prices: HashMap::new(),
            current_fdv: HashMap::new(),
        }
    }

    /// Register a token with its supply and decimals
    pub fn register_token(&mut self, mint: Pubkey, supply: u64, decimals: u8) {
        self.token_supplies.insert(mint, supply);
        self.token_decimals.insert(mint, decimals);
        self.recalculate_fdv(mint);
    }

    /// Update price for a token and recalculate FDV
    pub fn update_price(&mut self, mint: &Pubkey, price: f64) {
        self.current_prices.insert(*mint, price);
        self.recalculate_fdv(*mint);
    }

    /// Recalculate FDV for a token
    fn recalculate_fdv(&mut self, mint: Pubkey) {
        let supply = self.token_supplies.get(&mint);
        let decimals = self.token_decimals.get(&mint);
        let price = self.current_prices.get(&mint);

        if let (Some(&supply), Some(&decimals), Some(&price)) = (supply, decimals, price) {
            // Convert supply from base units to tokens
            let supply_tokens = supply as f64 / 10_f64.powi(decimals as i32);
            let fdv = price * supply_tokens;
            self.current_fdv.insert(mint, fdv);
        }
    }

    /// Get current FDV for a token in SOL
    pub fn get_fdv_sol(&self, mint: &Pubkey) -> Option<f64> {
        self.current_fdv.get(mint).copied()
    }

    /// Get current FDV for a token formatted as USD
    /// Note: This assumes a SOL price. In production, fetch real SOL price.
    pub fn get_fdv_usd(&self, mint: &Pubkey, sol_price: f64) -> Option<f64> {
        self.get_fdv_sol(mint).map(|fdv_sol| fdv_sol * sol_price)
    }

    /// Format FDV as a string
    pub fn format_fdv(fdv: f64) -> String {
        if fdv >= 1_000_000_000.0 {
            format!("${:.2}B", fdv / 1_000_000_000.0)
        } else if fdv >= 1_000_000.0 {
            format!("${:.2}M", fdv / 1_000_000.0)
        } else if fdv >= 1_000.0 {
            format!("${:.2}K", fdv / 1_000.0)
        } else {
            format!("${:.2}", fdv)
        }
    }
}

impl Default for FdvEngine {
    fn default() -> Self {
        Self::new()
    }
}
