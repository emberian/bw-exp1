//! BLACKWING Authentication Primitives
//!
//! Provides cryptographic primitives for authentication:
//! - Password hashing using Argon2id
//! - Session token generation and hashing
//!
//! Note: Axum middleware extractors live in the web layer since they
//! depend on specific state types.

mod password;
mod token;

pub use password::{hash_password, verify_password};
pub use token::{generate_token, hash_token, Token};
