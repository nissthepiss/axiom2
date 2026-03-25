use std::io::{self, Write};

use anyhow::{Context, Result};
use crate::types::Pubkey;
use tracing::info;

/// Default fallback mint (USDC on Solana)
pub const DEFAULT_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// Metaplex Token Metadata program ID
pub const METAPLEX_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

/// Token-2022 (Token Extensions) program ID
pub const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Token-2022 extension type for TokenMetadata
const EXT_TYPE_TOKEN_METADATA: u16 = 19;

/// Token metadata fetched at startup
#[derive(Debug, Clone)]
pub struct TokenMetadata {
    pub mint: Pubkey,
    pub symbol: String,
    pub name: String,
    pub supply: u64,
    pub decimals: u8,
}

/// Parsed Metaplex metadata account data
#[derive(Debug, Clone)]
struct MetaplexMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
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
    let account_info = client.get_account_info(mint).await
        .with_context(|| format!("Failed to fetch mint account for {}", mint))?;

    // Parse mint account data (SPL Token Mint struct layout):
    // - 0-35: mint_authority COption<Pubkey> (4 byte discriminator + 32 byte pubkey)
    // - 36-43: supply (u64, 8 bytes)
    // - 44: decimals (u8, 1 byte)
    // - 45: is_initialized (bool, 1 byte)
    // - 46-81: freeze_authority COption<Pubkey> (36 bytes)
    // Total: 82 bytes

    if account_info.data.len() < 82 {
        return Err(anyhow::anyhow!(
            "Invalid mint account data: expected at least 82 bytes, got {}",
            account_info.data.len()
        ));
    }

    let supply_bytes: [u8; 8] = account_info.data[36..44]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse supply bytes"))?;
    let supply = u64::from_le_bytes(supply_bytes);
    let decimals = account_info.data[44];

    // Determine metadata source based on token program
    let metadata = if account_info.owner == TOKEN_2022_PROGRAM_ID {
        // Token-2022: metadata is embedded in the mint account via TLV extensions
        info!("Detected Token-2022 mint, parsing embedded metadata extensions");
        parse_token2022_metadata(&account_info.data)
            .with_context(|| format!(
                "Failed to parse Token-2022 metadata extensions for mint {}",
                mint
            ))?
    } else {
        // Standard SPL Token: fetch Metaplex metadata from separate PDA
        fetch_metaplex_metadata(mint, rpc_url).await
            .with_context(|| {
                format!(
                    "Failed to resolve token metadata for mint {}\n\
                     The token may not have Metaplex metadata associated with it.\n\
                     Please verify the mint address is correct and the token has been initialized with metadata.",
                    mint
                )
            })?
    };

    Ok(TokenMetadata {
        mint: *mint,
        symbol: metadata.symbol,
        name: metadata.name,
        supply,
        decimals,
    })
}

/// Fetch Metaplex metadata for a token mint
async fn fetch_metaplex_metadata(mint: &Pubkey, rpc_url: &str) -> Result<MetaplexMetadata> {
    // Derive the Metaplex metadata PDA
    // Seeds: ["metadata", METAPLEX_PROGRAM_ID, mint]
    // Note: The seeds include the program_id bytes, NOT the pubkey string
    let metadata_program_id: Pubkey = METAPLEX_PROGRAM_ID.parse()
        .map_err(|_| anyhow::anyhow!("Invalid Metaplex program ID"))?;

    let seeds: Vec<&[u8]> = vec![
        b"metadata",
        metadata_program_id.as_ref(),
        mint.as_ref(),
    ];

    // The program_id for PDA derivation is the Metaplex program
    // derive_pda will hash: seeds + program_id + bump
    let metadata_pda = Pubkey::derive_pda(&seeds, &metadata_program_id);

    // Fetch the metadata account
    let client = crate::types::SolanaRpcClient::new(rpc_url);
    let account_info = client.get_account_info(&metadata_pda).await
        .with_context(|| format!("Metadata account not found for mint {}", mint))?;

    // Parse the metadata account data
    parse_metadata_account(&account_info.data)
}

/// Parse Token-2022 embedded metadata from mint account TLV extensions
fn parse_token2022_metadata(data: &[u8]) -> Result<MetaplexMetadata> {
    // Token-2022 mint layout:
    // - Bytes 0-81: standard Mint data (82 bytes)
    // - Bytes 82-164: padding (zeros)
    // - Byte 165: account type marker
    // - Bytes 166+: TLV extensions (type: u16 LE, length: u16 LE, data)

    let tlv_start = 166;
    if data.len() < tlv_start + 4 {
        return Err(anyhow::anyhow!("Mint account too short for Token-2022 extensions"));
    }

    let mut offset = tlv_start;
    while offset + 4 <= data.len() {
        let ext_type = u16::from_le_bytes([data[offset], data[offset + 1]]);
        let ext_len = u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;

        if ext_type == 0 && ext_len == 0 {
            // Skip padding
            continue;
        }

        if offset + ext_len > data.len() {
            break;
        }

        if ext_type == EXT_TYPE_TOKEN_METADATA {
            // TokenMetadata extension layout:
            // - update_authority: 32 bytes (Pubkey, zeroed = None)
            // - mint: 32 bytes (Pubkey)
            // - name: u32 LE length + UTF-8 data
            // - symbol: u32 LE length + UTF-8 data
            // - uri: u32 LE length + UTF-8 data
            let ext_data = &data[offset..offset + ext_len];
            let mut pos = 0;

            // Skip update_authority (32 bytes) and mint (32 bytes)
            pos += 64;
            if pos + 4 > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata too short for name"));
            }

            // Name
            let name_len = u32::from_le_bytes(ext_data[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            if pos + name_len > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata name extends beyond data"));
            }
            let name = String::from_utf8(ext_data[pos..pos + name_len].to_vec())
                .map_err(|_| anyhow::anyhow!("Failed to parse name as UTF-8"))?
                .trim_end_matches('\0')
                .to_string();
            pos += name_len;

            // Symbol
            if pos + 4 > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata too short for symbol"));
            }
            let symbol_len = u32::from_le_bytes(ext_data[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            if pos + symbol_len > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata symbol extends beyond data"));
            }
            let symbol = String::from_utf8(ext_data[pos..pos + symbol_len].to_vec())
                .map_err(|_| anyhow::anyhow!("Failed to parse symbol as UTF-8"))?
                .trim_end_matches('\0')
                .trim()
                .to_string();
            pos += symbol_len;

            // URI
            if pos + 4 > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata too short for uri"));
            }
            let uri_len = u32::from_le_bytes(ext_data[pos..pos + 4].try_into().unwrap()) as usize;
            pos += 4;
            if pos + uri_len > ext_data.len() {
                return Err(anyhow::anyhow!("Token-2022 metadata uri extends beyond data"));
            }
            let uri = String::from_utf8(ext_data[pos..pos + uri_len].to_vec())
                .map_err(|_| anyhow::anyhow!("Failed to parse uri as UTF-8"))?
                .trim_end_matches('\0')
                .to_string();

            if symbol.is_empty() {
                return Err(anyhow::anyhow!("Token symbol is empty in Token-2022 metadata"));
            }

            return Ok(MetaplexMetadata { name, symbol, uri });
        }

        offset += ext_len;
    }

    Err(anyhow::anyhow!("No TokenMetadata extension found in Token-2022 mint account"))
}

/// Parse a Metaplex metadata account data buffer
fn parse_metadata_account(data: &[u8]) -> Result<MetaplexMetadata> {
    // Metadata account structure (simplified):
    // - 0: key type (1 byte) - should be 4 for Metadata
    // - 1: update authority (32 bytes)
    // - 33: mint (32 bytes)
    // - 65: name length (4 bytes, u32 little-endian)
    // - 69: name (variable)
    // - X: symbol length (4 bytes, u32 little-endian)
    // - X+4: symbol (variable)
    // - Y: uri length (4 bytes, u32 little-endian)
    // - Y+4: uri (variable)

    if data.len() < 69 {
        return Err(anyhow::anyhow!(
            "Invalid metadata account data: too short ({} bytes)",
            data.len()
        ));
    }

    let mut offset = 0;

    // Key type (1 byte) - should be 4 for Metadata
    let key_type = data[offset];
    offset += 1;
    if key_type != 4 {
        return Err(anyhow::anyhow!(
            "Invalid metadata key type: expected 4, got {}",
            key_type
        ));
    }

    // Update authority (32 bytes)
    offset += 32;

    // Mint (32 bytes)
    offset += 32;

    // Name length (4 bytes, u32 little-endian)
    let name_length_bytes: [u8; 4] = data[offset..offset + 4]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse name length bytes"))?;
    let name_length = u32::from_le_bytes(name_length_bytes) as usize;
    offset += 4;

    // Name (variable)
    if offset + name_length > data.len() {
        return Err(anyhow::anyhow!("Invalid metadata: name extends beyond data"));
    }
    let name = String::from_utf8(data[offset..offset + name_length].to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to parse name as UTF-8"))?
        .trim_end_matches('\0')
        .to_string();
    offset += name_length;

    // Symbol length (4 bytes, u32 little-endian)
    if offset + 4 > data.len() {
        return Err(anyhow::anyhow!("Invalid metadata: symbol length extends beyond data"));
    }
    let symbol_length_bytes: [u8; 4] = data[offset..offset + 4]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse symbol length bytes"))?;
    let symbol_length = u32::from_le_bytes(symbol_length_bytes) as usize;
    offset += 4;

    // Symbol (variable)
    if offset + symbol_length > data.len() {
        return Err(anyhow::anyhow!("Invalid metadata: symbol extends beyond data"));
    }
    let symbol = String::from_utf8(data[offset..offset + symbol_length].to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to parse symbol as UTF-8"))?
        .trim_end_matches('\0')
        .trim()
        .to_string();
    offset += symbol_length;

    // URI length (4 bytes, u32 little-endian)
    if offset + 4 > data.len() {
        return Err(anyhow::anyhow!("Invalid metadata: uri length extends beyond data"));
    }
    let uri_length_bytes: [u8; 4] = data[offset..offset + 4]
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to parse uri length bytes"))?;
    let uri_length = u32::from_le_bytes(uri_length_bytes) as usize;
    offset += 4;

    // URI (variable)
    if offset + uri_length > data.len() {
        return Err(anyhow::anyhow!("Invalid metadata: uri extends beyond data"));
    }
    let uri = String::from_utf8(data[offset..offset + uri_length].to_vec())
        .map_err(|_| anyhow::anyhow!("Failed to parse uri as UTF-8"))?
        .trim_end_matches('\0')
        .to_string();

    // Validate that we got meaningful data
    if symbol.is_empty() {
        return Err(anyhow::anyhow!("Token symbol is empty in metadata"));
    }

    Ok(MetaplexMetadata {
        name,
        symbol,
        uri,
    })
}
