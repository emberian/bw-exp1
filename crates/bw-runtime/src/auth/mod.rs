//! Authentication module
//!
//! Provides password hashing, session token management, and Axum middleware.
//!
//! Note: Cryptographic primitives (password hashing, token generation) are in bw-auth.
//! This module re-exports them and provides Axum-specific middleware.

mod admin;
mod middleware;

// Re-export primitives from bw-auth
pub use bw_auth::{hash_password, verify_password, generate_token, hash_token, Token};

// Export middleware
pub use admin::{AdminAuth, is_admin};
pub use middleware::{AuthExtractor, OptionalAuth};
