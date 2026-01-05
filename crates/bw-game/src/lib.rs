//! BLACKWING Game Mechanics
//!
//! Core game state management, archetypes (data definitions), and systems.
//! This crate provides the foundation for game mechanics used by scripting and server.

pub mod state;
pub mod archetypes;
pub mod systems;

// Re-export derive macros from bw-scripting-macros
pub use bw_scripting_macros::{RhaiSerialize, RhaiDeserialize, RhaiSchema};

// Re-export commonly used types
pub use state::*;
pub use archetypes::{
    ArchetypeRegistry, ShipArchetype, WeaponArchetype, ShipStats, ShipWeapon,
    EffectArchetype, EffectType, StackingBehavior, EffectTrigger,
    AbilityArchetype, AbilityTarget, AbilityCost, AbilityEffect, AbilityRequirements,
    CargoArchetype, CargoCategory,
    FactionArchetype, FactionBehaviors, FactionCombatBonuses, FactionStandingRequirements,
};

/// Entity types that can have behaviors attached or be targets of mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Ship,
    Station,
    Sector,
    Mission,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ship => "ship",
            Self::Station => "station",
            Self::Sector => "sector",
            Self::Mission => "mission",
        }
    }
}
