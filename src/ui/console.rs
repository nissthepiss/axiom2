use crate::events::trade_event::{TradeEvent, TradeSide};
use crate::types::Pubkey;

/// Console renderer for trade events
pub struct ConsoleRenderer {
    token_symbol: String,
    token_name: String,
    last_fdv: Option<f64>,
    sol_price: f64,
}

impl ConsoleRenderer {
    pub fn new(symbol: String, name: String) -> Self {
        Self {
            token_symbol: symbol,
            token_name: name,
            last_fdv: None,
            sol_price: 150.0, // Default SOL price, should be fetched from oracle
        }
    }

    /// Set SOL price for USD calculations
    pub fn set_sol_price(&mut self, price: f64) {
        self.sol_price = price;
    }

    /// Print header with token info and current FDV
    pub fn print_header(&self, fdv_sol: Option<f64>) {
        let fdv_display = if let Some(fdv) = fdv_sol {
            let fdv_usd = fdv * self.sol_price;
            crate::engine::FdvEngine::format_fdv(fdv_usd)
        } else {
            "Calculating...".to_string()
        };

        println!("\n\x1b[1;36mTracking {}\x1b[0m", self.token_symbol);
        println!("\x1b[1;33mFDV: {}\x1b[0m", fdv_display);
        println!("\x1b[90m{}\x1b[0m", "─".repeat(40));
    }

    /// Print FDV header (alias for print_header)
    pub fn print_fdv_header(&self, fdv_sol: Option<f64>) {
        self.print_header(fdv_sol);
    }

    /// Update and re-print header with new FDV
    pub fn update_fdv(&mut self, fdv_sol: f64) {
        self.last_fdv = Some(fdv_sol);
        self.print_header(Some(fdv_sol));
    }

    /// Print a trade event
    pub fn print_trade(&self, trade: &TradeEvent) {
        let timestamp = Self::format_timestamp(trade.timestamp);
        let side_str = match trade.side {
            TradeSide::Buy => "\x1b[1;32mBUY\x1b[0m",
            TradeSide::Sell => "\x1b[1;31mSELL\x1b[0m",
        };
        let wallet = Self::shorten_address(trade.wallet);

        println!(
            "{} | {} {:.4} SOL | Wallet: {}",
            timestamp,
            side_str,
            trade.sol_amount,
            wallet
        );
    }

    /// Format timestamp as HH:MM:SS
    fn format_timestamp(ts: i64) -> String {
        let seconds = ts % 86400;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let secs = seconds % 60;

        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }

    /// Shorten a pubkey address
    fn shorten_address(pubkey: Pubkey) -> String {
        let addr = pubkey.to_string();
        if addr.len() > 8 {
            format!("{}...{}", &addr[..3], &addr[addr.len() - 3..])
        } else {
            addr
        }
    }

    /// Print startup banner
    pub fn print_banner(&self) {
        println!("\x1b[1;36m╔════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[1;36m║   Axiom Telemetry - Real-time CLI    \x1b[0m");
        println!("\x1b[1;36m╚════════════════════════════════════════╝\x1b[0m");
    }
}
