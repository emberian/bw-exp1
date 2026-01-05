//! Rate limiting middleware using governor
//!
//! Provides IP-based rate limiting for sensitive endpoints.

use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::{num::NonZeroU32, sync::Arc, time::{Duration, Instant}};

/// How long an IP entry can be idle before cleanup removes it.
const CLEANUP_IDLE_THRESHOLD: Duration = Duration::from_secs(300); // 5 minutes

/// IP-keyed rate limiter using governor.
#[derive(Clone)]
pub struct RateLimiter {
    /// Per-IP limiters with last-used timestamp for cleanup
    limiters: Arc<DashMap<String, (Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>, Instant)>>,
    /// Requests per window
    requests_per_window: NonZeroU32,
    /// Window duration
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(requests_per_window: u32, window: Duration) -> Self {
        Self {
            limiters: Arc::new(DashMap::new()),
            requests_per_window: NonZeroU32::new(requests_per_window).unwrap_or(NonZeroU32::MIN),
            window,
        }
    }

    /// Create a strict rate limiter for auth endpoints (10 req/min).
    pub fn auth() -> Self {
        Self::new(10, Duration::from_secs(60))
    }

    /// Create a moderate rate limiter for API endpoints (30 req/min).
    pub fn api() -> Self {
        Self::new(30, Duration::from_secs(60))
    }

    /// Get or create a rate limiter for the given IP, updating last-used time.
    fn get_limiter(&self, ip: &str) -> Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>> {
        let now = Instant::now();

        // Use entry API for atomic get-or-insert to avoid race conditions
        let mut entry = self.limiters.entry(ip.to_string()).or_insert_with(|| {
            let quota = Quota::with_period(self.window / self.requests_per_window.get())
                .expect("valid quota")
                .allow_burst(self.requests_per_window);
            let limiter = Arc::new(GovernorRateLimiter::direct(quota));
            (limiter, now)
        });

        // Update last-used time
        entry.1 = now;
        entry.0.clone()
    }

    /// Check if a request from the given IP should be allowed.
    pub fn check(&self, ip: &str) -> Result<(), ()> {
        let limiter = self.get_limiter(ip);
        limiter.check().map_err(|_| ())
    }

    /// Clean up old entries (call periodically).
    pub fn cleanup(&self) {
        // Remove entries that haven't been used within the idle threshold
        self.limiters.retain(|_, (_, last_used)| {
            last_used.elapsed() < CLEANUP_IDLE_THRESHOLD
        });
    }
}

/// Create rate limiter for auth endpoints.
pub fn auth_rate_limiter() -> RateLimiter {
    RateLimiter::auth()
}

/// Create rate limiter for API endpoints.
pub fn api_rate_limiter() -> RateLimiter {
    RateLimiter::api()
}
