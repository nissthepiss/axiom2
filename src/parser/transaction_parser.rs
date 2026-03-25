use crate::types::Pubkey;
use anyhow::Result;
use prost::Message;

/// WSOL mint address - treated as native SOL
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Trade event detected from balance changes
#[derive(Debug, Clone)]
pub struct TradeEvent {
    pub wallet: Pubkey,
    pub token_delta: i64,
    pub sol_delta: i64,
    pub is_buy: bool,
}

impl TradeEvent {
    pub fn new(wallet: Pubkey, token_delta: i64, sol_delta: i64) -> Self {
        let is_buy = token_delta > 0 && sol_delta < 0;
        TradeEvent {
            wallet,
            token_delta,
            sol_delta,
            is_buy,
        }
    }
}

/// Parsed transaction with account keys and balance changes
#[derive(Debug, Clone)]
pub struct ParsedTransaction {
    pub account_keys: Vec<Pubkey>,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_index: u32,
    pub mint: Pubkey,
    pub amount: u64,  // Raw amount (not UI amount)
    pub decimals: u8,
    pub owner: Option<Pubkey>,
}

/// Parse Solana transaction message from Yellowstone bytes
pub fn parse_transaction_message(transaction_bytes: &[u8]) -> Result<Vec<Pubkey>> {
    // Solana transaction message format:
    // - 1 byte: number of required signatures
    // - 1 byte: number of readonly signed accounts
    // - 1 byte: number of readonly unsigned accounts
    // - Then: account keys (32 bytes each, public keys)

    if transaction_bytes.is_empty() {
        return Ok(Vec::new());
    }

    let mut offset = 0;

    // Read header (3 bytes)
    if transaction_bytes.len() < 3 {
        return Ok(Vec::new());
    }
    offset += 3;

    // Read account count (Compact-u16)
    let (account_count, bytes_read) = read_compact_u16(&transaction_bytes[offset..])?;
    offset += bytes_read;

    // Read account keys
    let mut account_keys = Vec::new();
    for _ in 0..account_count {
        if offset + 32 > transaction_bytes.len() {
            break;
        }
        let key_bytes = &transaction_bytes[offset..offset + 32];
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(key_bytes);
        account_keys.push(Pubkey::from(key_array));
        offset += 32;
    }

    Ok(account_keys)
}

/// Parse transaction metadata from Yellowstone protobuf-encoded meta bytes
pub fn parse_transaction_meta(meta_bytes: &[u8]) -> Result<TransactionMeta> {
    use crate::stream::geyser::TransactionStatusMeta as ProtoMeta;

    let proto_meta = ProtoMeta::decode(meta_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to decode protobuf TransactionStatusMeta: {}", e))?;

    let pre_token_balances = proto_meta.pre_token_balances
        .iter()
        .filter_map(|tb| convert_token_balance(tb))
        .collect();

    let post_token_balances = proto_meta.post_token_balances
        .iter()
        .filter_map(|tb| convert_token_balance(tb))
        .collect();

    Ok(TransactionMeta {
        err: proto_meta.err.map(|e| e.err),
        fee: proto_meta.fee,
        pre_balances: proto_meta.pre_balances,
        post_balances: proto_meta.post_balances,
        pre_token_balances,
        post_token_balances,
    })
}

/// Convert a protobuf TokenBalance to our local TokenBalance
fn convert_token_balance(proto_tb: &crate::stream::geyser::TokenBalance) -> Option<TokenBalance> {
    let mint: Pubkey = proto_tb.mint.parse().ok()?;
    let owner = if proto_tb.owner.is_empty() {
        None
    } else {
        proto_tb.owner.parse().ok()
    };

    let ui_amount = proto_tb.ui_token_amount.as_ref()?;
    let amount: u64 = ui_amount.amount.parse().ok()?;
    let decimals = ui_amount.decimals as u8;

    Some(TokenBalance {
        account_index: proto_tb.account_index,
        mint,
        amount,
        decimals,
        owner,
    })
}

#[derive(Debug, Clone)]
pub struct TransactionMeta {
    pub err: Option<Vec<u8>>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
}

/// Read compact u16 (variable length integer)
fn read_compact_u16(data: &[u8]) -> Result<(u16, usize)> {
    if data.is_empty() {
        return Ok((0, 1));
    }

    let first_byte = data[0];
    match first_byte {
        0x00..=0x7f => Ok((first_byte as u16, 1)),
        0x80..=0xbf => {
            if data.len() < 2 {
                return Ok((0, 1));
            }
            let value = ((first_byte as u16 & 0x3f) << 8) | (data[1] as u16);
            Ok((value, 2))
        }
        0xc0..=0xff => {
            if data.len() < 3 {
                return Ok((0, 1));
            }
            // Use u32 to avoid overflow, then truncate to u16
            let value = (((first_byte as u32 & 0x3f) << 16) |
                        ((data[1] as u32) << 8) |
                        (data[2] as u32)) as u16;
            Ok((value, 3))
        }
        _ => Ok((0, 1)),
    }
}

/// Calculate balance deltas for a transaction
pub fn calculate_balance_deltas(
    transaction_bytes: &[u8],
    meta_bytes: &[u8],
    target_mint: &Pubkey,
) -> Result<Vec<TradeEvent>> {
    // Parse transaction message
    let account_keys = parse_transaction_message(transaction_bytes)?;

    // Parse transaction meta
    let meta = parse_transaction_meta(meta_bytes)?;

    if account_keys.is_empty() || meta.pre_balances.is_empty() {
        return Ok(Vec::new());
    }

    let wsol_mint: Pubkey = WSOL_MINT.parse().unwrap_or_default();

    // Build a map of owner -> (pre_token_amount, post_token_amount) for the target mint
    let mut wallet_token_deltas: std::collections::HashMap<Pubkey, (i64, i64)> = std::collections::HashMap::new();

    for tb in &meta.pre_token_balances {
        if tb.mint == *target_mint {
            if let Some(owner) = &tb.owner {
                let entry = wallet_token_deltas.entry(*owner).or_insert((0, 0));
                entry.0 += tb.amount as i64;
            }
        }
    }
    for tb in &meta.post_token_balances {
        if tb.mint == *target_mint {
            if let Some(owner) = &tb.owner {
                let entry = wallet_token_deltas.entry(*owner).or_insert((0, 0));
                entry.1 += tb.amount as i64;
            }
        }
    }

    // Build a map of owner -> WSOL delta (for swaps that use WSOL instead of native SOL)
    let mut wallet_wsol_deltas: std::collections::HashMap<Pubkey, i64> = std::collections::HashMap::new();

    for tb in &meta.pre_token_balances {
        if tb.mint == wsol_mint {
            if let Some(owner) = &tb.owner {
                *wallet_wsol_deltas.entry(*owner).or_insert(0) -= tb.amount as i64;
            }
        }
    }
    for tb in &meta.post_token_balances {
        if tb.mint == wsol_mint {
            if let Some(owner) = &tb.owner {
                *wallet_wsol_deltas.entry(*owner).or_insert(0) += tb.amount as i64;
            }
        }
    }

    // Build owner -> account_index map for SOL balance lookup
    let mut owner_to_index: std::collections::HashMap<Pubkey, usize> = std::collections::HashMap::new();
    for tb in meta.pre_token_balances.iter().chain(meta.post_token_balances.iter()) {
        if let Some(owner) = &tb.owner {
            // Find this owner in account_keys
            if let Some(idx) = account_keys.iter().position(|k| k == owner) {
                owner_to_index.insert(*owner, idx);
            }
        }
    }

    let mut trades = Vec::new();

    for (wallet, (pre_amount, post_amount)) in &wallet_token_deltas {
        let token_delta = post_amount - pre_amount;
        if token_delta == 0 {
            continue;
        }

        // Calculate SOL delta from native SOL balances (using owner's account index)
        let mut sol_delta: i64 = 0;
        if let Some(&idx) = owner_to_index.get(wallet) {
            if idx < meta.pre_balances.len() && idx < meta.post_balances.len() {
                sol_delta = meta.post_balances[idx] as i64 - meta.pre_balances[idx] as i64;
            }
        }

        // Add WSOL delta (many DEXes use WSOL for swaps)
        if let Some(&wsol_delta) = wallet_wsol_deltas.get(wallet) {
            sol_delta += wsol_delta;
        }

        // Filter out tiny movements
        const MIN_SOL_DELTA: i64 = 1_000; // 0.000001 SOL
        if sol_delta.abs() < MIN_SOL_DELTA {
            continue;
        }

        let is_buy = token_delta > 0 && sol_delta < 0;
        let is_sell = token_delta < 0 && sol_delta > 0;

        if is_buy || is_sell {
            trades.push(TradeEvent::new(*wallet, token_delta, sol_delta));
        }
    }

    Ok(trades)
}

/// Check if a mint is WSOL
pub fn is_wsol(mint: &Pubkey) -> bool {
    mint.to_string() == WSOL_MINT
}
