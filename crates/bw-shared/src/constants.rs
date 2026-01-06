//! Game constants
//!
//! Shared constants between server and client.

/// Server tick rate (ticks per second).
/// DEBUG: Set to 0.2 TPS (5 second ticks) for debugging
pub const TICK_RATE: u32 = 1; // Nominal value, actual rate is 0.2 TPS

/// Tick duration in milliseconds.
/// DEBUG: 5000ms = 0.2 TPS for debugging
pub const TICK_DURATION_MS: u64 = 5000;

/// Maximum ships per sector.
pub const MAX_SHIPS_PER_SECTOR: usize = 100;

/// Maximum players per sector.
pub const MAX_PLAYERS_PER_SECTOR: usize = 50;

/// Maximum active missions per sector.
pub const MAX_MISSIONS_PER_SECTOR: usize = 20;

/// Offline attack protection count.
pub const OFFLINE_ATTACK_LIMIT: i32 = 5;

/// Starting reputation for new players.
pub const STARTING_REPUTATION: i32 = 100;

/// Fame decay rate per decay tick.
pub const FAME_DECAY_RATE: i32 = 1;

/// Ticks between fame decay.
pub const FAME_DECAY_INTERVAL: u32 = 100;

/// Critical ammunition threshold (percentage).
pub const AMMO_CRITICAL_THRESHOLD: f32 = 15.0;

/// Critical fuel threshold (percentage).
pub const FUEL_CRITICAL_THRESHOLD: f32 = 10.0;

/// Neutral morale threshold.
pub const MORALE_NEUTRAL: f32 = 50.0;

/// WebSocket ping interval in seconds.
pub const WS_PING_INTERVAL: u64 = 30;

/// WebSocket timeout in seconds.
pub const WS_TIMEOUT: u64 = 60;

/// Mission expiry warning threshold in seconds.
pub const MISSION_EXPIRY_WARNING: u32 = 60;

/// Chat message max length.
pub const CHAT_MAX_LENGTH: usize = 500;

/// Squadron tag max length.
pub const SQUADRON_TAG_MAX_LENGTH: usize = 5;

/// Squadron name max length.
pub const SQUADRON_NAME_MAX_LENGTH: usize = 32;

/// Player name max length.
pub const PLAYER_NAME_MAX_LENGTH: usize = 24;

/// Ship name max length.
pub const SHIP_NAME_MAX_LENGTH: usize = 32;
