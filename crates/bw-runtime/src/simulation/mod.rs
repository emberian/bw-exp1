//! Game simulation / game loop

mod combat_processor;
mod game_loop;
pub mod metrics;
mod mission_executor;
mod npc_spawner;
pub mod script_hooks;
pub mod sim_config;
mod squadron_manager;
mod station_services;

pub use combat_processor::*;
pub use game_loop::*;
pub use mission_executor::*;
pub use npc_spawner::*;
pub use script_hooks::ScriptHooks;
pub use squadron_manager::*;
pub use station_services::*;
