use std::io::{self, Write};

use anyhow::Result;
use crate::types::Pubkey;
use tracing::info;

/// Default fallback mint (USDC on Solana)
pub const DEFAULT_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Token metadata fetched at startup
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub supply: u64,
    pub decimals: u8,
}

/// Prompt user for token mint via stdin
pub fn prompt_for_mint() -> Result<String> {
    print!("Enter token mint (press ENTER to use default): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let mint = input.trim().to_string();
    Ok(mint)
}

/// Get mint address - either from user input or default
pub fn resolve_mint() -> Result<String> {
    let input = prompt_for_mint()?;

    if input.is_empty() {
        Ok(DEFAULT_MINT.to_string())
    } else {
        Ok(input)
    }
}

/// Fetch token metadata from RPC
pub async fn fetch_token_metadata(mint: &Pubkey, rpc_url: &str) -> Result<TokenMetadata> {
    info!("Fetching token metadata from RPC: {}", rpc_url);

    let client = crate::types::SolanaRpcClient::new(rpc_url);

    // Fetch mint account info
    let account_info = client.get_account_info(mint).await?;

    // Parse mint account data
    // The mint account data structure:
    // - 0-4: mint authority option (32 bytes)
    // - 4-36: freeze authority option (32 bytes)
    // - 36-44: supply (u64)
    // - 44-45: decimals (u8)
    // - rest: is_initialized flag

    if account_info.data.len() < 45 {
        return Err(anyhow::anyhow!("Invalid mint account data"));
    }

    let supply = u64::from_le_bytes(
        account_info.data[36..44]
            .try_into()
            .map_err(|_| anyhow::anyhow!("Failed to parse supply"))?,
    );
    let decimals = account_info.data[44];

    // Try to fetch Metaplex metadata
    let metadata = fetch_metaplex_metadata(mint, rpc_url).await.unwrap_or_else(|_| {
        (
            "UNKNOWN".to_string(),
            format!("Token-{}", &mint.to_string()[..8]),
        )
    });

    Ok(TokenMetadata {
        mint: *mint,
        symbol: metadata.0,
        name: metadata.1,
        supply,
        decimals,
    })
}

/// Fetch Metaplex metadata for a token mint
async fn fetch_metaplex_metadata(mint: &Pubkey, _rpc_url: &str) -> Result<(String, String)> {
    // Derive Metaplex metadata PDA
    // metadata_program_id = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    // seeds = ["metadata", mint_id, "edition"]

    let _metadata_program_id = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

    // For now, return default values
    // TODO: Implement proper Metaplex metadata PDA derivation and fetching
    Ok((
        "UNKNOWN".to_string(),
        format!("Token-{}", &mint.to_string()[..8]),
    ))
}
