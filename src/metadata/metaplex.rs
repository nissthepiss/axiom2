use borsh::{BorshDeserialize, BorshSerialize};
// Use re-exported types from yellowstone-grpc-proto
use yellowstone_grpc_proto::prelude::{
    solana_client::rpc_client::RpcClient,
    solana_sdk::pubkey::Pubkey,
};
use std::str::FromStr;

use super::resolver::{derive_metadata_pda, MetadataError};

/// Metaplex Metadata Account structure (simplified)
/// Based on Metaplex Token Metadata program
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct MetadataAccount {
    pub key: Key,
    pub update_authority: Option<Pubkey>,
    pub mint: Pubkey,
    pub data: Data,
    // Primary sale happened, is mutable, etc. omitted for brevity
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct Data {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub seller_fee_basis_points: u16,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, PartialEq, Eq)]
pub enum Key {
    Uninitialized,
    MetadataV1,
    // Other variants omitted
}

/// Fetch and parse Metaplex metadata account
pub fn fetch_metadata(
    client: &RpcClient,
    mint: &Pubkey,
) -> Result<(String, String, String), MetadataError> {
    let (metadata_pda, _bump) = derive_metadata_pda(mint);

    let account = client
        .get_account(&metadata_pda)
        .map_err(|e| MetadataError::RpcError(e.to_string()))?
        .ok_or(MetadataError::MetadataNotFound)?;

    // Metaplex metadata accounts start with a 1-byte key discriminator
    // We need to skip some bytes based on the actual structure
    let data = &account.data;

    // The first byte is the key
    if data.is_empty() {
        return Err(MetadataError::InvalidAccountData);
    }

    // For simplicity, we'll do a basic parse
    // In production, you'd use the full Metaplex types
    // The structure after the key includes:
    // - update_authority (32 bytes, optional)
    // - mint (32 bytes)
    // - data (name, symbol, uri, seller_fee_basis_points)

    let mut offset = 1; // Skip key byte

    // Skip update authority (32 bytes + 1 byte for option)
    offset += 33;

    // Skip mint (32 bytes) - we already know it
    offset += 32;

    // Now we're at the Data struct
    // Each string is prefixed with a 4-byte length

    // Parse name (length + string)
    if data.len() < offset + 4 {
        return Err(MetadataError::InvalidAccountData);
    }
    let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    if data.len() < offset + name_len {
        return Err(MetadataError::InvalidAccountData);
    }
    let name = String::from_utf8_lossy(&data[offset..offset + name_len])
        .trim_end_matches('\0')
        .to_string();
    offset += name_len;

    // Parse symbol
    if data.len() < offset + 4 {
        return Err(MetadataError::InvalidAccountData);
    }
    let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    if data.len() < offset + symbol_len {
        return Err(MetadataError::InvalidAccountData);
    }
    let symbol = String::from_utf8_lossy(&data[offset..offset + symbol_len])
        .trim_end_matches('\0')
        .to_string();
    offset += symbol_len;

    // Parse URI
    if data.len() < offset + 4 {
        return Err(MetadataError::InvalidAccountData);
    }
    let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    if data.len() < offset + uri_len {
        return Err(MetadataError::InvalidAccountData);
    }
    let uri = String::from_utf8_lossy(&data[offset..offset + uri_len])
        .trim_end_matches('\0')
        .to_string();

    Ok((name, symbol, uri))
}
