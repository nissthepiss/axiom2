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
            .map_err(|_| "Invalid base58".to_string())
            .and_then(|bytes| {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Ok(Pubkey(arr))
                } else {
                    Err("Invalid pubkey length".to_string())
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
