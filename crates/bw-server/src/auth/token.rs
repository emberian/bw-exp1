//! Session token generation and hashing
//!
//! Tokens are cryptographically random strings that are:
//! - Generated using a CSPRNG (cryptographically secure random number generator)
//! - Base64-URL encoded for safe transmission
//! - Hashed with SHA256 before storage (we never store raw tokens)

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// A session token with both raw and hashed forms.
#[derive(Debug, Clone)]
pub struct Token {
    /// The raw token to send to the client (never stored).
    pub raw: String,
    /// The hashed token for database storage.
    pub hash: String,
}

/// Generate a cryptographically secure session token.
///
/// Returns a Token struct containing:
/// - `raw`: The token to send to the client (32 bytes, base64-URL encoded)
/// - `hash`: The SHA256 hash of the raw token (for storage)
pub fn generate_token() -> Token {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = URL_SAFE_NO_PAD.encode(bytes);
    let hash = hash_token(&raw);
    Token { raw, hash }
}

/// Hash a token for secure storage.
///
/// We store the hash, not the raw token, so even if the database is
/// compromised, the attacker cannot impersonate users.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = generate_token();

        // Raw token should be 43 characters (32 bytes base64-URL encoded without padding)
        assert_eq!(token.raw.len(), 43);

        // Hash should be 43 characters (32 bytes SHA256 hash, base64-URL encoded)
        assert_eq!(token.hash.len(), 43);

        // Hash should match when we hash the raw token
        assert_eq!(hash_token(&token.raw), token.hash);
    }

    #[test]
    fn test_tokens_are_unique() {
        let token1 = generate_token();
        let token2 = generate_token();

        assert_ne!(token1.raw, token2.raw);
        assert_ne!(token1.hash, token2.hash);
    }

    #[test]
    fn test_hash_is_deterministic() {
        let token = "test_token_string";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        assert_eq!(hash1, hash2);
    }
}
