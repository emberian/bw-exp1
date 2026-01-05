//! Ship archetype definitions
//!
//! Ship archetypes define the base stats and properties of ship classes.

use rhai::{Array, Dynamic, Map};
use serde::{Deserialize, Serialize};
use crate::RhaiDeserialize;

/// Base stats for a ship archetype.
#[derive(Debug, Clone, Serialize, Deserialize, RhaiDeserialize)]
pub struct ShipStats {
    #[rhai(default = 10.0)]
    pub attack: f32,
    #[rhai(default = 10.0)]
    pub defense: f32,
    #[rhai(default = 50.0)]
    pub speed: f32,
    #[rhai(default = 50.0)]
    pub shield_capacity: f32,
    #[rhai(default = 100.0)]
    pub sensor_range: f32,
    #[rhai(default = 50)]
    pub cargo_capacity: u32,
    #[rhai(default = 5.0)]
    pub fuel_per_sector: f32,
    #[rhai(default = 1.0)]
    pub ammo_per_attack: f32,
}

impl Default for ShipStats {
    fn default() -> Self {
        Self {
            attack: 10.0,
            defense: 10.0,
            speed: 50.0,
            shield_capacity: 50.0,
            sensor_range: 100.0,
            cargo_capacity: 50,
            fuel_per_sector: 5.0,
            ammo_per_attack: 1.0,
        }
    }
}

/// Weapon definition within a ship archetype.
#[derive(Debug, Clone, Serialize, Deserialize, RhaiDeserialize)]
pub struct ShipWeapon {
    #[rhai(rename = "type")]
    pub weapon_type: String,
    #[rhai(default = 10.0)]
    pub damage: f32,
    #[rhai(default = 0.7)]
    pub accuracy: f32,
    #[rhai(default = 1.0)]
    pub ammo_cost: f32,
}

/// A ship archetype defining a class of ships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipArchetype {
    /// Unique identifier (e.g., "patrol_corvette")
    pub id: String,

    /// Display name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Tier/level (for progression)
    pub tier: u32,

    /// Base stats
    pub stats: ShipStats,

    /// Default weapons
    pub weapons: Vec<ShipWeapon>,

    /// Available upgrade slots
    pub upgrade_slots: Vec<String>,

    /// Whether players can use this class
    pub is_player_class: bool,

    /// Whether this is a hostile NPC class
    pub is_hostile: bool,

    /// Behavior scripts to attach to NPCs of this class
    pub behaviors: Vec<String>,

    /// Unlock requirements (optional)
    pub unlock: Option<UnlockRequirements>,
}

/// Requirements to unlock a ship class.
#[derive(Debug, Clone, Default, Serialize, Deserialize, RhaiDeserialize)]
pub struct UnlockRequirements {
    pub fame: Option<i32>,
    pub reputation: Option<i32>,
    pub credits: Option<i64>,
    pub missions_completed: Option<u32>,
}

impl ShipArchetype {
    /// Parse a ship archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());

        // Parse stats - can be nested or flat
        let stats = if let Some(stats_val) = map.get("stats") {
            ShipStats::from_dynamic(stats_val).unwrap_or_default()
        } else {
            // Try to parse stats from top-level keys
            ShipStats {
                attack: get_f32(&map, "attack").unwrap_or(10.0),
                defense: get_f32(&map, "defense").unwrap_or(10.0),
                speed: get_f32(&map, "speed").unwrap_or(50.0),
                shield_capacity: get_f32(&map, "shield_capacity").unwrap_or(50.0),
                sensor_range: get_f32(&map, "sensor_range").unwrap_or(100.0),
                cargo_capacity: get_u32(&map, "cargo_capacity").unwrap_or(50),
                fuel_per_sector: get_f32(&map, "fuel_per_sector").unwrap_or(5.0),
                ammo_per_attack: get_f32(&map, "ammo_per_attack").unwrap_or(1.0),
            }
        };

        // Parse weapons array
        let weapons: Vec<ShipWeapon> = map.get("weapons")
            .and_then(|w| w.clone().try_cast::<Array>())
            .map(|arr: Array| {
                arr.into_iter()
                    .filter_map(|w| ShipWeapon::from_dynamic(&w))
                    .collect()
            })
            .unwrap_or_default();

        // Parse upgrade slots
        let upgrade_slots: Vec<String> = map.get("upgrade_slots")
            .and_then(|s| s.clone().try_cast::<Array>())
            .map(|arr: Array| {
                arr.into_iter()
                    .filter_map(|s: Dynamic| s.into_string().ok())
                    .collect()
            })
            .unwrap_or_default();

        // Parse behaviors
        let behaviors: Vec<String> = map.get("behaviors")
            .and_then(|b| b.clone().try_cast::<Array>())
            .map(|arr: Array| {
                arr.into_iter()
                    .filter_map(|b: Dynamic| b.into_string().ok())
                    .collect()
            })
            .unwrap_or_default();

        // Parse unlock requirements
        let unlock = map.get("unlock")
            .and_then(|u| UnlockRequirements::from_dynamic(u));

        Some(Self {
            id,
            name,
            description: get_string(&map, "description"),
            tier: get_u32(&map, "tier").unwrap_or(1),
            stats,
            weapons,
            upgrade_slots,
            is_player_class: get_bool(&map, "is_player_class").unwrap_or(false),
            is_hostile: get_bool(&map, "is_hostile").unwrap_or(false),
            behaviors,
            unlock,
        })
    }
}

// Helper functions for parsing Rhai maps
fn get_string(map: &Map, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.clone().into_string().ok())
}

fn get_f32(map: &Map, key: &str) -> Option<f32> {
    map.get(key).and_then(|v| {
        v.as_float().ok().map(|f| f as f32)
            .or_else(|| v.as_int().ok().map(|i| i as f32))
    })
}

fn get_u32(map: &Map, key: &str) -> Option<u32> {
    map.get(key).and_then(|v| v.as_int().ok().map(|i| i as u32))
}

fn get_bool(map: &Map, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ship_archetype() {
        let mut map = Map::new();
        map.insert("id".into(), Dynamic::from("test_ship"));
        map.insert("name".into(), Dynamic::from("Test Ship"));
        map.insert("attack".into(), Dynamic::from(30.0_f64));
        map.insert("defense".into(), Dynamic::from(20.0_f64));
        map.insert("is_player_class".into(), Dynamic::from(true));

        let archetype = ShipArchetype::from_dynamic(Dynamic::from(map)).unwrap();
        assert_eq!(archetype.id, "test_ship");
        assert_eq!(archetype.name, "Test Ship");
        assert_eq!(archetype.stats.attack, 30.0);
        assert!(archetype.is_player_class);
    }

    // Schema tests moved to bw-scripting (requires schema module)
}
