//! Game simulation / game loop

mod game_loop;
mod npc_spawner;
mod combat_processor;
mod mission_executor;
mod station_services;
mod squadron_manager;

pub use game_loop::*;
pub use npc_spawner::*;
pub use combat_processor::*;
pub use mission_executor::*;
pub use station_services::*;
pub use squadron_manager::*;
