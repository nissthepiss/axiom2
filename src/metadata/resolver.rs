// Use re-exported types from yellowstone-grpc-proto
use yellowstone_grpc_proto::prelude::{
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        account::Account,
        pubkey::Pubkey,
    },
};
use thiserror::Error;

/// Context information for a tracked token
#[derive(Debug, Clone)]
pub struct TokenContext {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub supply: u64,
    pub decimals: u8,
    pub metadata_uri: String,
}

#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Account not found: {0}")]
    AccountNotFound(Pubkey),

    #[error("Invalid account data")]
    InvalidAccountData,

    #[error("Metadata not found")]
    MetadataNotFound,
}

/// Fetch mint account information from Solana RPC
pub fn fetch_mint_account(
    client: &RpcClient,
    mint: &Pubkey,
) -> Result<(u64, u8), MetadataError> {
    let account = client
        .get_account(mint)
        .map_err(|e| MetadataError::RpcError(e.to_string()))?
        .ok_or(MetadataError::AccountNotFound(*mint))?;

    // Mint account is 82 bytes minimum
    if account.data.len() < 82 {
        return Err(MetadataError::InvalidAccountData);
    }

    // Parse mint account data
    // Offset 36: supply (u64)
    // Offset 44: decimals (u8)
    let supply = u64::from_le_bytes(
        account.data[36..44]
            .try_into()
            .map_err(|_| MetadataError::InvalidAccountData)?,
    );

    let decimals = account.data[44];

    Ok((supply, decimals))
}

/// Derive Metaplex metadata PDA
pub fn derive_metadata_pda(mint: &Pubkey) -> (Pubkey, u8) {
    // Metaplex Metadata program ID
    let metadata_program_id = Pubkey::from_str_const("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

    // "metadata" string as seed
    let seeds = &[
        b"metadata".as_ref(),
        metadata_program_id.as_ref(),
        mint.as_ref(),
    ];

    Pubkey::find_program_address(seeds, &metadata_program_id)
}
