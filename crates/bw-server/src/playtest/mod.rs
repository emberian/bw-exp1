//! GM Playtest Mode
//!
//! Allows GMs to create isolated forks of game state for testing changes
//! before deploying them to the live game.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      PlaytestManager                         │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
//! │  │ Playtest #1  │  │ Playtest #2  │  │ Playtest #3  │  ...  │
//! │  └──────────────┘  └──────────────┘  └──────────────┘       │
//! └─────────────────────────────────────────────────────────────┘
//!                            │
//!                     Message Router
//!                            │
//!               ┌────────────┴────────────┐
//!               │                         │
//!        Live GameState            PlaytestInstance
//!        (all other players)       (GM + invited)
//! ```
//!
//! # Key Features
//!
//! - **Full State Fork**: Creates isolated copies of ships, players, sectors
//! - **Shared Static Data**: Factions shared via Arc (no duplication)
//! - **Session-Based**: Playtests are not persisted across server restarts
//! - **Script Compatible**: PlaytestInstance implements StateProvider
//!
//! # Usage
//!
//! ```ignore
//! // Create a playtest
//! let config = ForkConfig::single_sector(sector_id);
//! let builder = PlaytestBuilder::new(gm_id, "Test".into(), tick, config);
//! let instance = builder.build(factions, faction_tags);
//!
//! // Fork state from live
//! game_state.fork_to_playtest(&mut instance, &builder.config());
//!
//! // Register with manager
//! let instance = playtest_manager.register(instance)?;
//!
//! // Players join
//! playtest_manager.join_playtest(playtest_id, player_id, ship_id, sector_id)?;
//!
//! // When done
//! playtest_manager.destroy(playtest_id)?;
//! ```

mod config;
mod instance;
mod manager;
mod router;
mod simulation;
mod state_provider;

pub use config::{ForkConfig, PlaytestError, PromoteConfig, PromoteResult};
pub use instance::{PlaytestInstance, PlaytestParticipant, PlaytestSectorInstance};
pub use manager::{PlaytestBuilder, PlaytestManager};
pub use router::MessageDestination;
pub use simulation::spawn_playtest_loop;
