use crate::types::Pubkey;
use std::collections::HashMap;

/// Wallet-specific token balance changes
#[derive(Debug, Clone, Default)]
pub struct WalletDeltas {
    pub owner: Pubkey,
    pub sol_delta: i64,      // Native SOL lamports (includes fees)
    pub wsol_delta: i64,     // WSOL token delta
    pub token_deltas: HashMap<Pubkey, i64>, // Other tokens
}

impl WalletDeltas {
    pub fn new(owner: Pubkey) -> Self {
        Self {
            owner,
            ..Default::default()
        }
    }

    /// Get effective SOL delta (native + WSOL)
    pub fn effective_sol_delta(&self) -> i64 {
        self.sol_delta + self.wsol_delta
    }

    /// Get token delta for specific mint
    pub fn get_token_delta(&self, mint: &Pubkey) -> i64 {
        *self.token_deltas.get(mint).unwrap_or(&0)
    }
}

/// Parse token balance changes from transaction metadata
/// For now, this is a placeholder that will be updated once we have proper transaction data
pub fn extract_balance_deltas(
    _tx: &(),  // Will be replaced with actual transaction type
) -> Vec<WalletDeltas> {
    // TODO: Implement actual parsing once we have transaction data structure
    Vec::new()
}
