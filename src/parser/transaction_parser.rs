use crate::types::Pubkey;
use anyhow::Result;
use serde::Deserialize;

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

/// Parse transaction metadata from Yellowstone meta bytes
pub fn parse_transaction_meta(meta_bytes: &[u8]) -> Result<TransactionMeta> {
    // For now, we'll try to parse as JSON first
    // Yellowstone may provide meta as JSON or binary format

    if let Ok(meta_str) = std::str::from_utf8(meta_bytes) {
        if let Ok(json_meta) = parse_json_meta(meta_str) {
            return Ok(json_meta);
        }
    }

    // Fallback: try binary parsing
    parse_binary_meta(meta_bytes)
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

/// Parse metadata from JSON format (Yellowstone may provide this)
fn parse_json_meta(meta_str: &str) -> Result<TransactionMeta> {
    #[derive(Deserialize)]
    struct JsonMeta {
        #[serde(default)]
        err: Option<serde_json::Value>,
        #[serde(default)]
        fee: u64,
        #[serde(default)]
        preBalances: Vec<u64>,
        #[serde(default)]
        postBalances: Vec<u64>,
        #[serde(default)]
        preTokenBalances: Option<Vec<JsonTokenBalance>>,
        #[serde(default)]
        postTokenBalances: Option<Vec<JsonTokenBalance>>,
    }

    #[derive(Deserialize)]
    struct JsonTokenBalance {
        accountIndex: u32,
        mint: String,
        uiTokenAmount: JsonUiTokenAmount,
        owner: Option<String>,
    }

    #[derive(Deserialize)]
    struct JsonUiTokenAmount {
        amount: String,
        decimals: u32,
        uiAmount: f64,
    }

    let json: JsonMeta = serde_json::from_str(meta_str)?;

    let pre_token_balances = json.preTokenBalances
        .unwrap_or_default()
        .into_iter()
        .filter_map(|tb| {
            let mint = tb.mint.parse().ok()?;
            let amount = tb.uiTokenAmount.amount.parse().ok()?;
            Some(TokenBalance {
                account_index: tb.accountIndex,
                mint,
                amount,
                decimals: tb.uiTokenAmount.decimals as u8,
                owner: tb.owner.and_then(|o| o.parse().ok()),
            })
        })
        .collect();

    let post_token_balances = json.postTokenBalances
        .unwrap_or_default()
        .into_iter()
        .filter_map(|tb| {
            let mint = tb.mint.parse().ok()?;
            let amount = tb.uiTokenAmount.amount.parse().ok()?;
            Some(TokenBalance {
                account_index: tb.accountIndex,
                mint,
                amount,
                decimals: tb.uiTokenAmount.decimals as u8,
                owner: tb.owner.and_then(|o| o.parse().ok()),
            })
        })
        .collect();

    Ok(TransactionMeta {
        err: None,
        fee: json.fee,
        pre_balances: json.preBalances,
        post_balances: json.postBalances,
        pre_token_balances,
        post_token_balances,
    })
}

/// Parse metadata from binary format
fn parse_binary_meta(_meta_bytes: &[u8]) -> Result<TransactionMeta> {
    // This is a simplified binary parser
    // In production, you'd want to use the full Solana binary format

    // For now, return empty metadata
    Ok(TransactionMeta {
        err: None,
        fee: 0,
        pre_balances: Vec::new(),
        post_balances: Vec::new(),
        pre_token_balances: Vec::new(),
        post_token_balances: Vec::new(),
    })
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

    // If we don't have account keys or balances, return empty
    if account_keys.is_empty() || meta.pre_balances.is_empty() {
        return Ok(Vec::new());
    }

    let mut trades = Vec::new();

    // Track processed wallets to avoid duplicates
    let mut processed_wallets = std::collections::HashSet::new();

    // Find token balance changes for the target mint
    for pre_token in &meta.pre_token_balances {
        if pre_token.mint != *target_mint {
            continue;
        }

        let account_index = pre_token.account_index as usize;

        // Find matching post token balance
        let post_amount = meta.post_token_balances
            .iter()
            .find(|pt| pt.account_index == pre_token.account_index && pt.mint == *target_mint)
            .map(|pt| pt.amount as i64)
            .unwrap_or(0);

        let pre_amount = pre_token.amount as i64;
        let token_delta = post_amount - pre_amount;

        // Skip if no token change
        if token_delta == 0 {
            continue;
        }

        // Get wallet address
        let wallet = if account_index < account_keys.len() {
            account_keys[account_index]
        } else if let Some(owner) = &pre_token.owner {
            *owner
        } else {
            continue;
        };

        // Skip if already processed this wallet
        if !processed_wallets.insert(wallet) {
            continue;
        }

        // Calculate SOL delta
        let sol_delta = if account_index < meta.pre_balances.len() && account_index < meta.post_balances.len() {
            let pre_sol = meta.pre_balances[account_index] as i64;
            let post_sol = meta.post_balances[account_index] as i64;
            post_sol - pre_sol
        } else {
            0
        };

        // Filter out tiny SOL movements (fees)
        const MIN_SOL_DELTA: i64 = 1_000_000; // 0.001 SOL in lamports
        if sol_delta.abs() < MIN_SOL_DELTA {
            continue;
        }

        // Detect swap direction
        let is_buy = token_delta > 0 && sol_delta < 0;
        let is_sell = token_delta < 0 && sol_delta > 0;

        if is_buy || is_sell {
            trades.push(TradeEvent::new(wallet, token_delta, sol_delta));
        }
    }

    // Also check post_token_balances for new token holders
    for post_token in &meta.post_token_balances {
        if post_token.mint != *target_mint {
            continue;
        }

        // Skip if we already processed this account
        if processed_wallets.len() > meta.pre_token_balances.len() {
            // Check if this account was in pre balances
            let account_index = post_token.account_index as usize;
            let wallet = if account_index < account_keys.len() {
                account_keys[account_index]
            } else if let Some(owner) = &post_token.owner {
                *owner
            } else {
                continue;
            };

            if !processed_wallets.contains(&wallet) {
                // This is a new token holder
                let token_delta = post_token.amount as i64;

                // Calculate SOL delta
                let sol_delta = if account_index < meta.pre_balances.len() && account_index < meta.post_balances.len() {
                    let pre_sol = meta.pre_balances[account_index] as i64;
                    let post_sol = meta.post_balances[account_index] as i64;
                    post_sol - pre_sol
                } else {
                    0
                };

                // Filter out tiny SOL movements
                const MIN_SOL_DELTA: i64 = 1_000_000;
                if sol_delta.abs() < MIN_SOL_DELTA {
                    continue;
                }

                let is_buy = token_delta > 0 && sol_delta < 0;
                if is_buy {
                    trades.push(TradeEvent::new(wallet, token_delta, sol_delta));
                }
            }
        }
    }

    Ok(trades)
}

/// Check if a mint is WSOL
pub fn is_wsol(mint: &Pubkey) -> bool {
    mint.to_string() == WSOL_MINT
}
