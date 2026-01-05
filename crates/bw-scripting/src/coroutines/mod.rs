//! Coroutine system for scripts
//!
//! Allows scripts to yield control and resume later, enabling multi-tick behaviors
//! like "patrol for 5 seconds, then attack". Since Rhai doesn't have native async,
//! we implement cooperative multitasking through yield markers.

mod scheduler;

pub use scheduler::*;

use std::cell::RefCell;
use rhai::{Dynamic, Map};
use uuid::Uuid;

// Thread-local storage for yield requests from the currently executing script.
thread_local! {
    pub static YIELD_REQUEST: RefCell<Option<YieldRequest>> = const { RefCell::new(None) };
}

/// A suspended script execution that can be resumed later.
#[derive(Clone)]
pub struct Coroutine {
    /// Unique identifier for this coroutine
    pub id: Uuid,
    /// Path to the script file
    pub script_path: String,
    /// Function to call when resuming
    pub function_name: String,
    /// Current state of the coroutine
    pub state: CoroutineState,
    /// Preserved scope for resumption (serialized as Map for Clone)
    pub local_vars: Map,
    /// Value to pass when resuming (from yield return)
    pub resume_value: Option<Dynamic>,
    /// Tick when this coroutine was created
    pub created_at: u64,
    /// Tick when this coroutine should resume (for tick-based waiting)
    pub resume_at: Option<u64>,
    /// Entity this coroutine belongs to (for behavior scripts)
    pub owner_entity_id: Option<Uuid>,
    /// Sector context
    pub sector_id: Option<Uuid>,
}

impl Coroutine {
    /// Create a new coroutine.
    pub fn new(
        script_path: impl Into<String>,
        function_name: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            script_path: script_path.into(),
            function_name: function_name.into(),
            state: CoroutineState::Ready,
            local_vars: Map::new(),
            resume_value: None,
            created_at,
            resume_at: None,
            owner_entity_id: None,
            sector_id: None,
        }
    }

    /// Set the owner entity.
    pub fn with_owner(mut self, entity_id: Uuid) -> Self {
        self.owner_entity_id = Some(entity_id);
        self
    }

    /// Set the sector context.
    pub fn with_sector(mut self, sector_id: Uuid) -> Self {
        self.sector_id = Some(sector_id);
        self
    }

    /// Check if the coroutine is ready to run.
    pub fn is_ready(&self, current_tick: u64) -> bool {
        match self.state {
            CoroutineState::Ready => true,
            CoroutineState::WaitingForTicks => {
                self.resume_at.map(|t| current_tick >= t).unwrap_or(false)
            }
            _ => false,
        }
    }
}

impl std::fmt::Debug for Coroutine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Coroutine")
            .field("id", &self.id)
            .field("script_path", &self.script_path)
            .field("function_name", &self.function_name)
            .field("state", &self.state)
            .field("created_at", &self.created_at)
            .field("resume_at", &self.resume_at)
            .finish()
    }
}

/// State of a coroutine.
#[derive(Debug, Clone, PartialEq)]
pub enum CoroutineState {
    /// Ready to run (not yet started or just resumed)
    Ready,
    /// Currently executing
    Running,
    /// Waiting for a specific number of ticks
    WaitingForTicks,
    /// Waiting for a specific event type
    WaitingForEvent(String),
    /// Waiting for next frame (next tick)
    WaitingForNextFrame,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed(String),
}

/// Request from a script to yield control.
#[derive(Debug, Clone)]
pub struct YieldRequest {
    /// Type of yield
    pub yield_type: YieldType,
    /// Additional data for the yield
    pub data: Map,
}

/// Types of yield operations.
#[derive(Debug, Clone)]
pub enum YieldType {
    /// Yield for a number of game ticks (at 10 TPS)
    Ticks(u64),
    /// Yield for a number of seconds (converted to ticks)
    Seconds(f64),
    /// Yield until next game tick
    NextFrame,
    /// Yield until a specific event type occurs
    Event(String),
    /// Schedule a callback function to run later (fire and forget)
    Schedule { delay_ticks: u64, callback: String },
}

impl YieldRequest {
    /// Create a tick-based yield request.
    pub fn ticks(count: u64) -> Self {
        Self {
            yield_type: YieldType::Ticks(count),
            data: Map::new(),
        }
    }

    /// Create a seconds-based yield request.
    pub fn seconds(secs: f64) -> Self {
        Self {
            yield_type: YieldType::Seconds(secs),
            data: Map::new(),
        }
    }

    /// Create a next-frame yield request.
    pub fn next_frame() -> Self {
        Self {
            yield_type: YieldType::NextFrame,
            data: Map::new(),
        }
    }

    /// Create an event-based yield request.
    pub fn event(event_type: impl Into<String>) -> Self {
        Self {
            yield_type: YieldType::Event(event_type.into()),
            data: Map::new(),
        }
    }
}

/// Result of executing a coroutine step.
#[derive(Debug, Clone)]
pub enum CoroutineExecResult {
    /// Script completed normally with a return value
    Completed(Dynamic),
    /// Script yielded and should be resumed later
    Yielded(YieldRequest),
    /// Script failed with an error
    Failed(String),
}

/// Result from the scheduler after processing a tick.
#[derive(Debug)]
pub struct CoroutineTickResult {
    /// Coroutines that completed this tick
    pub completed: Vec<Uuid>,
    /// Coroutines that failed this tick
    pub failed: Vec<(Uuid, String)>,
    /// Number of coroutines still pending
    pub pending_count: usize,
}

/// Check if the last script execution yielded.
pub fn take_yield_request() -> Option<YieldRequest> {
    YIELD_REQUEST.with(|req| req.borrow_mut().take())
}

/// Clear any pending yield request.
pub fn clear_yield_request() {
    YIELD_REQUEST.with(|req| {
        *req.borrow_mut() = None;
    });
}
