//! Hash helper functions for use with derivative crate.
//!
//! Provides stable hash implementations for floating-point types
//! which don't implement Hash natively due to IEEE NaN issues.

use std::hash::{Hash, Hasher};

/// Hash an f32 by its bit representation.
///
/// This provides stable, deterministic hashing for floats.
/// Used with `#[derivative(Hash(hash_with = "hash_f32"))]`.
pub fn hash_f32<H: Hasher>(val: &f32, state: &mut H) {
    val.to_bits().hash(state);
}

/// Hash an f64 by its bit representation.
///
/// This provides stable, deterministic hashing for floats.
/// Used with `#[derivative(Hash(hash_with = "hash_f64"))]`.
pub fn hash_f64<H: Hasher>(val: &f64, state: &mut H) {
    val.to_bits().hash(state);
}
