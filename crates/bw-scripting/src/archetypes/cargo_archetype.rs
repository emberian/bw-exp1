//! Cargo archetype definitions
//!
//! Cargo archetypes define types of tradeable goods and their properties.
//! They're loaded from scripts (scripts/definitions/cargo.rhai).

use rhai::{Array, Dynamic, Map};
use serde::{Deserialize, Serialize};

/// Category of cargo for UI grouping and trade rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CargoCategory {
    /// Basic consumable resources (fuel, food, etc.).
    #[default]
    Consumable,
    /// Industrial goods (ore, metals, components).
    Industrial,
    /// High-value goods (rare earths, luxuries).
    HighValue,
    /// Illegal contraband.
    Contraband,
    /// Military equipment.
    Military,
    /// Data/information.
    Data,
}

impl CargoCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "consumable" => CargoCategory::Consumable,
            "industrial" => CargoCategory::Industrial,
            "high_value" | "highvalue" | "luxury" => CargoCategory::HighValue,
            "contraband" | "illegal" => CargoCategory::Contraband,
            "military" | "weapons" => CargoCategory::Military,
            "data" | "information" => CargoCategory::Data,
            _ => CargoCategory::Consumable,
        }
    }
}

/// A cargo archetype defining a type of tradeable good.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoArchetype {
    /// Unique identifier (e.g., "fuel", "ore", "rare_earths").
    pub id: String,

    /// Display name.
    pub name: String,

    /// Description.
    pub description: Option<String>,

    /// Base price at standard markets.
    pub base_price: i64,

    /// Weight per unit (affects cargo capacity).
    pub weight: u32,

    /// Whether this cargo is legal to trade openly.
    pub legal: bool,

    /// Price volatility (0.0 = stable, 1.0 = highly volatile).
    pub volatility: f32,

    /// Penalty if caught with contraband (for illegal cargo).
    pub contraband_penalty: i32,

    /// Category for UI grouping and trade rules.
    pub category: CargoCategory,

    /// Special effects when carrying this cargo (e.g., "radioactive", "perishable").
    pub special_effects: Vec<String>,

    /// Icon identifier for UI.
    pub icon: Option<String>,

    /// Tier/rarity level (1 = common, 5 = extremely rare).
    pub tier: i32,

    /// Supply abundance at stations (affects availability).
    pub abundance: f32,

    /// Demand multiplier at different station types.
    pub demand_modifiers: Map,
}

impl Default for CargoArchetype {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            base_price: 100,
            weight: 1,
            legal: true,
            volatility: 0.1,
            contraband_penalty: 0,
            category: CargoCategory::Consumable,
            special_effects: vec![],
            icon: None,
            tier: 1,
            abundance: 1.0,
            demand_modifiers: Map::new(),
        }
    }
}

impl CargoArchetype {
    /// Parse a cargo archetype from a Rhai Dynamic value.
    pub fn from_dynamic(value: Dynamic) -> Option<Self> {
        let map = value.try_cast::<Map>()?;

        let id = get_string(&map, "id")?;
        let name = get_string(&map, "name").unwrap_or_else(|| id.clone());

        // Parse category
        let category = get_string(&map, "category")
            .map(|s| CargoCategory::from_str(&s))
            .unwrap_or_default();

        // Parse special effects
        let special_effects = get_string_array(&map, "special_effects");

        // Parse demand modifiers (station_type -> multiplier)
        let demand_modifiers = map.get("demand_modifiers")
            .and_then(|v| v.clone().try_cast::<Map>())
            .unwrap_or_default();

        Some(Self {
            id,
            name,
            description: get_string(&map, "description"),
            base_price: get_i64(&map, "base_price").unwrap_or(100),
            weight: get_i32(&map, "weight").unwrap_or(1) as u32,
            legal: get_bool(&map, "legal").unwrap_or(true),
            volatility: get_f32(&map, "volatility").unwrap_or(0.1),
            contraband_penalty: get_i32(&map, "contraband_penalty").unwrap_or(0),
            category,
            special_effects,
            icon: get_string(&map, "icon"),
            tier: get_i32(&map, "tier").unwrap_or(1),
            abundance: get_f32(&map, "abundance").unwrap_or(1.0),
            demand_modifiers,
        })
    }

    /// Check if this cargo is contraband.
    pub fn is_contraband(&self) -> bool {
        !self.legal || self.category == CargoCategory::Contraband
    }

    /// Check if this cargo has special effects.
    pub fn has_special_effects(&self) -> bool {
        !self.special_effects.is_empty()
    }

    /// Check if cargo has a specific special effect.
    pub fn has_effect(&self, effect: &str) -> bool {
        self.special_effects.iter().any(|e| e == effect)
    }

    /// Calculate adjusted price based on volatility and a random factor.
    pub fn calculate_price(&self, volatility_factor: f32) -> i64 {
        let adjustment = 1.0 + (volatility_factor * self.volatility);
        (self.base_price as f32 * adjustment) as i64
    }

    /// Get demand multiplier for a station type.
    pub fn get_demand_multiplier(&self, station_type: &str) -> f32 {
        self.demand_modifiers
            .get(station_type)
            .and_then(|v| v.clone().try_cast::<f64>().map(|f| f as f32))
            .unwrap_or(1.0)
    }
}

// Helper functions for parsing Rhai maps
fn get_string(map: &Map, key: &str) -> Option<String> {
    map.get(key).and_then(|v| v.clone().into_string().ok())
}

fn get_i32(map: &Map, key: &str) -> Option<i32> {
    map.get(key).and_then(|v| v.as_int().ok().map(|i| i as i32))
}

fn get_i64(map: &Map, key: &str) -> Option<i64> {
    map.get(key).and_then(|v| v.as_int().ok())
}

fn get_f32(map: &Map, key: &str) -> Option<f32> {
    map.get(key).and_then(|v| {
        v.as_float().ok().map(|f| f as f32)
            .or_else(|| v.as_int().ok().map(|i| i as f32))
    })
}

fn get_bool(map: &Map, key: &str) -> Option<bool> {
    map.get(key).and_then(|v| v.as_bool().ok())
}

fn get_string_array(map: &Map, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(|v| v.clone().try_cast::<Array>())
        .map(|arr| {
            arr.into_iter()
                .filter_map(|e| e.into_string().ok())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_category_from_str() {
        assert_eq!(CargoCategory::from_str("consumable"), CargoCategory::Consumable);
        assert_eq!(CargoCategory::from_str("industrial"), CargoCategory::Industrial);
        assert_eq!(CargoCategory::from_str("high_value"), CargoCategory::HighValue);
        assert_eq!(CargoCategory::from_str("contraband"), CargoCategory::Contraband);
        assert_eq!(CargoCategory::from_str("unknown"), CargoCategory::Consumable);
    }

    #[test]
    fn test_is_contraband() {
        let legal = CargoArchetype { legal: true, category: CargoCategory::Industrial, ..Default::default() };
        assert!(!legal.is_contraband());

        let illegal = CargoArchetype { legal: false, ..Default::default() };
        assert!(illegal.is_contraband());

        let contraband = CargoArchetype { category: CargoCategory::Contraband, ..Default::default() };
        assert!(contraband.is_contraband());
    }

    #[test]
    fn test_from_dynamic() {
        let mut map = Map::new();
        map.insert("id".into(), Dynamic::from("test_cargo"));
        map.insert("base_price".into(), Dynamic::from(500_i64));
        map.insert("weight".into(), Dynamic::from(2_i64));
        map.insert("legal".into(), Dynamic::from(false));
        map.insert("category".into(), Dynamic::from("contraband"));

        let cargo = CargoArchetype::from_dynamic(Dynamic::from(map)).unwrap();
        assert_eq!(cargo.id, "test_cargo");
        assert_eq!(cargo.base_price, 500);
        assert_eq!(cargo.weight, 2);
        assert!(!cargo.legal);
        assert_eq!(cargo.category, CargoCategory::Contraband);
    }

    #[test]
    fn test_calculate_price() {
        let cargo = CargoArchetype { base_price: 100, volatility: 0.5, ..Default::default() };

        // No volatility factor
        assert_eq!(cargo.calculate_price(0.0), 100);

        // Positive volatility
        assert_eq!(cargo.calculate_price(0.5), 125);

        // Negative volatility
        assert_eq!(cargo.calculate_price(-0.5), 75);
    }
}
