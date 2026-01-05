//! Weapon archetype definitions
//!
//! Weapon archetypes define weapon types and their properties.

use rhai::{Array, Dynamic, Map};
use serde::{Deserialize, Serialize};

/// A weapon archetype defining a type of weapon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponArchetype {
    /// Unique identifier (e.g., "railgun", "plasma_cannon")
    pub id: String,

    /// Display name
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Base damage
    pub damage: f32,

    /// Base accuracy (0.0 - 1.0)
    pub accuracy: f32,

    /// Ammo cost per shot
    pub ammo_cost: f32,

    /// Range modifier (1.0 = standard)
    pub range_modifier: f32,

    /// Special effects (e.g., "shield_piercing", "area_damage")
    pub effects: Vec<String>,

    /// Weapon category for slot compatibility
    pub category: String,

    /// Tier/level requirement
    pub tier: u32,

    /// Cost to purchase
    pub cost: i64,
}

impl Default for WeaponArchetype {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            damage: 10.0,
            accuracy: 0.7,
            ammo_cost: 1.0,
            range_modifier: 1.0,
            effects: Vec::new(),
            category: "weapon".to_string(),
            tier: 1,
            cost: 1000,
        }
    }
}

impl WeaponArchetype {
    /// Parse a weapon archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());

        // Parse effects array
        let effects: Vec<String> = map.get("effects")
            .and_then(|e| e.clone().try_cast::<Array>())
            .map(|arr: Array| {
                arr.into_iter()
                    .filter_map(|e: Dynamic| e.into_string().ok())
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            id,
            name,
            description: get_string(&map, "description"),
            damage: get_f32(&map, "damage").unwrap_or(10.0),
            accuracy: get_f32(&map, "accuracy").unwrap_or(0.7),
            ammo_cost: get_f32(&map, "ammo_cost").unwrap_or(1.0),
            range_modifier: get_f32(&map, "range_modifier").unwrap_or(1.0),
            effects,
            category: get_string(&map, "category").unwrap_or_else(|| "weapon".to_string()),
            tier: get_u32(&map, "tier").unwrap_or(1),
            cost: get_i64(&map, "cost").unwrap_or(1000),
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

fn get_i64(map: &Map, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| v.as_int().ok())
}

fn get_u32(map: &Map, key: &str) -> Option<u32> {
    map.get(key).and_then(|v| v.as_int().ok().map(|i| i as u32))
}
