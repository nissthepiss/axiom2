use crate::types::Pubkey;
use std::collections::HashMap;

use super::transaction_decoder::{extract_balance_deltas, WalletDeltas};

/// Filter for performance - check if tracked mint appears in transaction
/// For now, placeholder that will be updated with proper transaction data
pub fn contains_tracked_mint(
    _tx: &(),  // Will be replaced with actual transaction type
    _tracked_mint: &Pubkey,
) -> bool {
    // TODO: Implement actual filtering once we have transaction data structure
    // For now, return true to process all transactions for debugging
    true
}

/// Process transaction and extract aggregated wallet deltas
pub struct BalanceDeltaResult {
    pub wallet_deltas: HashMap<Pubkey, WalletDeltas>,
}

impl BalanceDeltaResult {
    pub fn process_transaction(_tx: &()) -> Self {
        let deltas = extract_balance_deltas(_tx);

        let mut wallet_map = HashMap::new();
        for delta in deltas {
            wallet_map.insert(delta.owner, delta);
        }

        Self {
            wallet_deltas: wallet_map,
        }
    }

    /// Get all wallets that had non-zero delta for a specific token
    pub fn get_wallets_with_token_delta(&self, mint: &Pubkey) -> Vec<&WalletDeltas> {
        self.wallet_deltas
            .values()
            .filter(|w| w.get_token_delta(mint) != 0)
            .collect()
    }
}
