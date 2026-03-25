use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Minimal Solana Pubkey implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub const fn new(pubkey: &[u8; 32]) -> Self {
        Pubkey(*pubkey)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Default for Pubkey {
    fn default() -> Self {
        Pubkey([0u8; 32])
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(&self.0).into_string())
    }
}

impl FromStr for Pubkey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        bs58::decode(s)
            .into_vec()
            .map_err(|e| format!("Invalid base58: {}", e))
            .and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Ok(Pubkey(arr))
                } else {
                    Err(format!("Invalid pubkey length: {}", bytes.len()))
                }
            })
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }
}

impl TryFrom<&[u8]> for Pubkey {
    type Error = String;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(bytes);
            Ok(Pubkey(arr))
        } else {
            Err("Invalid pubkey length".to_string())
        }
    }
}

impl TryFrom<Vec<u8>> for Pubkey {
    type Error = String;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Pubkey::try_from(bytes.as_slice())
    }
}

impl Pubkey {
    pub fn from_str_const(s: &str) -> Self {
        Self::from_str(s).unwrap_or_else(|_| Self::default())
    }
}

impl PartialEq<[u8; 32]> for Pubkey {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.0 == other
    }
}

impl Pubkey {
    /// Derive a Program Derived Address (PDA) from seeds and a program ID.
    /// Implements the full Solana PDA derivation: tries bumps 255..0,
    /// hashes seeds + [bump] + program_id + "ProgramDerivedAddress",
    /// and returns the first result that is NOT on the ed25519 curve.
    pub fn derive_pda(seeds: &[&[u8]], program_id: &Pubkey) -> Self {
        use sha2::{Digest, Sha256};
        use curve25519_dalek::edwards::CompressedEdwardsY;

        for bump in (0u8..=255).rev() {
            let mut hasher = Sha256::new();
            for seed in seeds {
                hasher.update(seed);
            }
            hasher.update(&[bump]);
            hasher.update(program_id.as_ref());
            hasher.update(b"ProgramDerivedAddress");

            let hash: [u8; 32] = hasher.finalize().into();

            // PDA must NOT be on the ed25519 curve
            let point = CompressedEdwardsY(hash);
            if point.decompress().is_none() {
                return Pubkey(hash);
            }
        }

        // Should never happen in practice
        Pubkey([0u8; 32])
    }
}

/// Simple Solana RPC client for metadata fetching
pub struct SolanaRpcClient {
    rpc_url: String,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub data: Vec<u8>,
    pub owner: String,
}

impl SolanaRpcClient {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_account_info(&self, pubkey: &Pubkey) -> anyhow::Result<AccountInfo> {
        use base64::Engine;

        #[derive(serde::Serialize)]
        struct Request {
            jsonrpc: String,
            id: u64,
            method: String,
            params: Vec<serde_json::Value>,
        }

        let request = Request {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "getAccountInfo".to_string(),
            params: vec![
                serde_json::to_value(pubkey.to_string()).unwrap(),
                serde_json::json!({"encoding": "base64"})
            ],
        };

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;

        let response_text = response.text().await?;

        // Parse using serde_json::Value for flexibility
        let json: serde_json::Value = serde_json::from_str(&response_text)?;

        // Check if account exists
        if json["result"]["value"].is_null() {
            return Err(anyhow::anyhow!("Account not found: {}", pubkey));
        }

        let data_array = json["result"]["value"]["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Missing data array"))?;

        let base64_str = data_array.first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing base64 data string"))?;

        let decoded_data = base64::engine::general_purpose::STANDARD
            .decode(base64_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode base64: {}", e))?;

        let owner = json["result"]["value"]["owner"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(AccountInfo { data: decoded_data, owner })
    }
}
