use crate::events::TelemetryEvent;
use crate::types::Pubkey;

/// Normalize and validate telemetry events.
/// Retained for future use in the enrichment pipeline.
pub struct TradeNormalizer;

impl TradeNormalizer {
    /// Validate a trade event
    pub fn validate(event: &TelemetryEvent) -> bool {
        match event {
            TelemetryEvent::Trade { sol_amount, token_amount, price, wallet, .. } => {
                *sol_amount > 0.0 && *token_amount > 0.0 && *price > 0.0 && *wallet != Pubkey::default()
            }
            _ => true,
        }
    }
}
