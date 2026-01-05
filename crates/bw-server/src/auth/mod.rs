//! Authentication module
//!
//! Provides password hashing, session token management, and Axum middleware.

mod admin;
mod middleware;
mod password;
mod token;

pub use admin::AdminAuth;
pub use middleware::AuthExtractor;
pub use password::{hash_password, verify_password};
pub use token::{generate_token, hash_token, Token};
