use serde::{Deserialize, Serialize};
use crate::types::Pubkey;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub side: TradeSide,
    pub sol_amount: f64,
    pub token_amount: f64,
    pub wallet: Pubkey,
    pub price: f64,
    pub slot: u64,
    pub timestamp: i64,
    pub tx_signature: String,
}

impl TradeEvent {
    pub fn new(
        mint: Pubkey,
        side: TradeSide,
        sol_amount: f64,
        token_amount: f64,
        wallet: Pubkey,
        price: f64,
        slot: u64,
        timestamp: i64,
        tx_signature: String,
    ) -> Self {
        Self {
            mint,
            side,
            sol_amount,
            token_amount,
            wallet,
            price,
            slot,
            timestamp,
            tx_signature,
        }
    }
}
