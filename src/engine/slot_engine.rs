use crate::events::TelemetryEvent;
use std::collections::HashMap;

/// Slot engine for price stabilization
///
/// Collects trades within the same slot and finalizes price when slot changes.
/// This prevents FDV flickering during rapid trading.
pub struct SlotEngine {
    current_slot: u64,
    finalized_prices: HashMap<u64, f64>,
}

impl SlotEngine {
    pub fn new() -> Self {
        Self {
            current_slot: 0,
            finalized_prices: HashMap::new(),
        }
    }

    pub fn current_slot(&self) -> u64 {
        self.current_slot
    }

    pub fn get_slot_price(&self, slot: u64) -> Option<f64> {
        self.finalized_prices.get(&slot).copied()
    }

    pub fn latest_price(&self) -> Option<f64> {
        self.finalized_prices.values().last().copied()
    }
}

impl Default for SlotEngine {
    fn default() -> Self {
        Self::new()
    }
}
