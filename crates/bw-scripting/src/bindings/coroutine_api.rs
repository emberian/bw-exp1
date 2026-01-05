//! Coroutine API bindings for Rhai
//!
//! Exposes yield functions that allow scripts to pause execution
//! and resume later. These work by setting a thread-local yield request
//! that the scheduler checks after each script execution.

use rhai::{Engine, Dynamic, Map};

use crate::coroutines::{YIELD_REQUEST, YieldRequest, YieldType};

/// Register coroutine API functions with the engine.
pub fn register(engine: &mut Engine) {
    // === Yield functions ===

    // yield_ticks(count: i64) -> ()
    // Pause execution for a number of game ticks
    engine.register_fn("yield_ticks", |ticks: i64| {
        let ticks = ticks.max(0) as u64;
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::Ticks(ticks),
                data: Map::new(),
            });
        });
    });

    // yield_seconds(seconds: f64) -> ()
    // Pause execution for a number of seconds (converted to ticks)
    engine.register_fn("yield_seconds", |seconds: f64| {
        let seconds = seconds.max(0.0);
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::Seconds(seconds),
                data: Map::new(),
            });
        });
    });

    // yield_frame() -> ()
    // Pause execution until the next game tick
    engine.register_fn("yield_frame", || {
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::NextFrame,
                data: Map::new(),
            });
        });
    });

    // yield_until_event(event_type: String) -> ()
    // Pause execution until a specific event occurs
    // When resumed, the event data will be available as __resume_value
    engine.register_fn("yield_until_event", |event_type: String| {
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::Event(event_type),
                data: Map::new(),
            });
        });
    });

    // schedule(delay_ticks: i64, callback: String) -> ()
    // Schedule a function to be called after a delay (fire and forget)
    // Does NOT pause the current execution
    engine.register_fn("schedule", |delay_ticks: i64, callback: String| {
        let delay = delay_ticks.max(0) as u64;
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::Schedule {
                    delay_ticks: delay,
                    callback,
                },
                data: Map::new(),
            });
        });
    });

    // schedule_seconds(delay: f64, callback: String) -> ()
    // Schedule with delay in seconds
    engine.register_fn("schedule_seconds", |delay: f64, callback: String| {
        let delay_ticks = (delay.max(0.0) * bw_shared::constants::TICK_RATE as f64).ceil() as u64;
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = Some(YieldRequest {
                yield_type: YieldType::Schedule {
                    delay_ticks,
                    callback,
                },
                data: Map::new(),
            });
        });
    });

    // === Resume value access ===

    // get_resume_value() -> Dynamic
    // Get the value passed when resuming from a yield
    // (e.g., event data when resuming from yield_until_event)
    engine.register_fn("get_resume_value", || -> Dynamic {
        // This is a placeholder - the actual value is injected into scope
        // by the scheduler before resuming. Scripts should access __resume_value directly.
        Dynamic::UNIT
    });

    // === Utility functions ===

    // is_yielding() -> bool
    // Check if a yield has been requested (useful for debugging)
    engine.register_fn("is_yielding", || -> bool {
        YIELD_REQUEST.with(|req| req.borrow().is_some())
    });

    // cancel_yield() -> ()
    // Cancel a pending yield (useful if logic changes mid-function)
    engine.register_fn("cancel_yield", || {
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = None;
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_request_set() {
        // Clear any existing request
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = None;
        });

        // Create engine and register
        let mut engine = Engine::new();
        register(&mut engine);

        // Run a script that yields
        let _ = engine.eval::<()>("yield_ticks(5)");

        // Check yield was requested
        let has_yield = YIELD_REQUEST.with(|req| req.borrow().is_some());
        assert!(has_yield);

        // Clean up
        YIELD_REQUEST.with(|req| {
            *req.borrow_mut() = None;
        });
    }
}
