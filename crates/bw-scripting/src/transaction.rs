//! Transaction support for scripts
//!
//! Provides atomic multi-step operations with auto-rollback on failure.
//!
//! ```rhai
//! fn execute_trade(player, station, cargo_type, qty) {
//!     let tx = begin_transaction();
//!
//!     let price = calculate_price(station, cargo_type) * qty;
//!     player.spend_credits(price);
//!     player.ship.add_cargo(cargo_type, qty, price);
//!
//!     tx.commit();  // All-or-nothing
//!     // If script throws before commit(), mutations are discarded
//! }
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use rhai::{Engine, CustomType, TypeBuilder};

use crate::context::with_accessor;

/// Transaction context for atomic script operations.
///
/// When created, records a checkpoint of pending mutations.
/// If `commit()` is not called before the transaction is dropped,
/// all mutations added after the checkpoint are rolled back.
///
/// # Clone Safety
///
/// Clones share the same committed state via Arc<AtomicBool>, so
/// committing or rolling back one clone affects all clones.
#[derive(Debug, Clone)]
pub struct TransactionContext {
    /// The checkpoint (mutation index) when transaction started.
    checkpoint: usize,
    /// Whether commit() was called (shared across clones).
    committed: Arc<AtomicBool>,
}

impl TransactionContext {
    /// Create a new transaction context.
    pub fn new() -> Self {
        let checkpoint = with_accessor(|accessor| {
            accessor.begin_transaction()
        }).unwrap_or(0);

        Self {
            checkpoint,
            committed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Commit the transaction, making all mutations permanent.
    pub fn commit(&mut self) -> bool {
        // Use compare_exchange to ensure only one commit succeeds
        if self.committed.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst
        ).is_err() {
            return true; // Already committed
        }

        with_accessor(|accessor| {
            accessor.commit_transaction();
        });

        true
    }

    /// Explicitly rollback the transaction.
    pub fn rollback(&mut self) {
        // Use compare_exchange to ensure only one rollback succeeds
        if self.committed.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst
        ).is_err() {
            return; // Already committed or rolled back
        }

        with_accessor(|accessor| {
            accessor.rollback_to_checkpoint(self.checkpoint);
        });
    }

    /// Check if transaction is committed.
    pub fn is_committed(&mut self) -> bool {
        self.committed.load(Ordering::SeqCst)
    }
}

impl Default for TransactionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TransactionContext {
    fn drop(&mut self) {
        // Auto-rollback if not committed (use compare_exchange to ensure only one drop does rollback)
        if self.committed.compare_exchange(
            false,
            true,
            Ordering::SeqCst,
            Ordering::SeqCst
        ).is_ok() {
            tracing::debug!(
                script = true,
                "Transaction not committed, rolling back {} mutations",
                with_accessor(|accessor| {
                    accessor.pending_mutation_count().saturating_sub(self.checkpoint)
                }).unwrap_or(0)
            );

            with_accessor(|accessor| {
                accessor.rollback_to_checkpoint(self.checkpoint);
            });
        }
    }
}

// Implement CustomType for Rhai registration
impl CustomType for TransactionContext {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("TransactionContext")
            .with_fn("commit", Self::commit)
            .with_fn("rollback", Self::rollback)
            .with_get("is_committed", Self::is_committed);
    }
}

/// Register transaction support with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register the TransactionContext type
    engine.build_type::<TransactionContext>();

    // Register the begin_transaction() function
    engine.register_fn("begin_transaction", || -> TransactionContext {
        TransactionContext::new()
    });
}
