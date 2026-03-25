use crate::events::trade_event::{TradeEvent, TradeSide};
use crate::types::Pubkey;

// ANSI 256-color codes
const ORANGE: &str = "\x1b[38;5;208m";
const ORANGE_BOLD: &str = "\x1b[1;38;5;208m";
const GREY: &str = "\x1b[38;5;245m";
const WHITE_BOLD: &str = "\x1b[1;37m";
const GREEN_BOLD: &str = "\x1b[1;32m";
const RED_BOLD: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

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
            sol_price: 150.0,
        }
    }

    /// Get current SOL price
    pub fn sol_price(&self) -> f64 {
        self.sol_price
    }

    /// Set SOL price for USD calculations
    pub fn set_sol_price(&mut self, price: f64) {
        self.sol_price = price;
    }

    /// Print startup banner — clean minimal style
    pub fn print_banner(&self) {
        println!();
        println!("  {ORANGE_BOLD}AXIOM TELEMETRY{RESET}");
        println!("  {GREY}{}{RESET}", "─".repeat(45));
    }

    /// Print header: mint, SYMBOL - Name, FDV, separator
    pub fn print_header(&self, fdv_sol: Option<f64>) {
        let fdv_display = if let Some(fdv) = fdv_sol {
            let fdv_usd = fdv * self.sol_price;
            format!("{WHITE_BOLD}{}{RESET}", crate::engine::FdvEngine::format_fdv(fdv_usd))
        } else {
            format!("{GREY}Calculating...{RESET}")
        };

        println!("  {ORANGE_BOLD}{}{RESET} {GREY}-{RESET} {WHITE_BOLD}{}{RESET}", self.token_symbol, self.token_name);
        println!("  {GREY}Market Cap (FDV):{RESET} {fdv_display}");
        println!("  {GREY}{}{RESET}", "─".repeat(45));
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
            TradeSide::Buy => format!("{GREEN_BOLD} BUY{RESET}"),
            TradeSide::Sell => format!("{RED_BOLD}SELL{RESET}"),
        };
        let wallet = Self::shorten_address(trade.wallet);

        println!(
            "  {GREY}{}{RESET}  {}  {WHITE_BOLD}{:.4} SOL{RESET}  {GREY}{}{RESET}",
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
            format!("{}...{}", &addr[..4], &addr[addr.len() - 4..])
        } else {
            addr
        }
    }
}
