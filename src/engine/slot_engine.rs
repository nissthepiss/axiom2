use crate::events::trade_event::TradeEvent;
use std::collections::HashMap;

/// Slot engine for price stabilization
///
/// Collects trades within the same slot and finalizes price when slot changes.
/// This prevents FDV flickering during rapid trading.
pub struct SlotEngine {
    current_slot: u64,
    slot_buffer: Vec<TradeEvent>,
    finalized_prices: HashMap<u64, f64>,
}

impl SlotEngine {
    pub fn new() -> Self {
        Self {
            current_slot: 0,
            slot_buffer: Vec::new(),
            finalized_prices: HashMap::new(),
        }
    }

    /// Process a trade event
    /// Returns Some(price) if a slot was finalized, None otherwise
    pub fn process_trade(&mut self, trade: &TradeEvent) -> Option<f64> {
        // If slot changed, finalize previous slot
        let result = if trade.slot != self.current_slot {
            if !self.slot_buffer.is_empty() {
                let finalized_price = self.finalize_slot();
                self.current_slot = trade.slot;
                Some(finalized_price)
            } else {
                self.current_slot = trade.slot;
                None
            }
        } else {
            None
        };

        // Add trade to current slot buffer
        self.slot_buffer.push(trade.clone());
        result
    }

    /// Finalize current slot and return the official price
    fn finalize_slot(&mut self) -> f64 {
        // Use the last trade's price in the slot as the official price
        let price = self
            .slot_buffer
            .last()
            .map(|t| t.price)
            .unwrap_or(0.0);

        self.finalized_prices.insert(self.current_slot, price);
        self.slot_buffer.clear();

        price
    }

    /// Force finalize current slot (e.g., on shutdown)
    pub fn force_finalize(&mut self) -> Option<f64> {
        if self.slot_buffer.is_empty() {
            None
        } else {
            Some(self.finalize_slot())
        }
    }

    /// Get current slot
    pub fn current_slot(&self) -> u64 {
        self.current_slot
    }

    /// Get finalized price for a slot
    pub fn get_slot_price(&self, slot: u64) -> Option<f64> {
        self.finalized_prices.get(&slot).copied()
    }

    /// Get latest finalized price
    pub fn latest_price(&self) -> Option<f64> {
        self.finalized_prices.values().last().copied()
    }
}

impl Default for SlotEngine {
    fn default() -> Self {
        Self::new()
    }
}
