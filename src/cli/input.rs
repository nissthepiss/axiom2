use clap::Parser;
use crate::types::Pubkey;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(name = "axiom_telemetry")]
#[command(about = "Real-time Solana swap telemetry CLI", long_about = None)]
pub struct Cli {
    /// Token mint address to track
    #[arg(short, long)]
    pub mint: String,

    /// Yellowstone gRPC endpoint
    #[arg(short, long, default_value = "chainstack")]
    pub endpoint: String,

    /// Solana RPC endpoint
    #[arg(short, long, default_value = "http://localhost:8899")]
    pub rpc: String,
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Invalid pubkey: {0}")]
    InvalidPubkey(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl Cli {
    pub fn parse_mint(&self) -> Result<Pubkey, CliError> {
        self.mint
            .parse::<Pubkey>()
            .map_err(|_| CliError::InvalidPubkey(self.mint.clone()))
    }
}
