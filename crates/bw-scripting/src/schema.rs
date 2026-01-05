//! Schema types for script validation.
//!
//! These types are used by the `#[derive(RhaiSchema)]` macro to generate
//! static schema information that validators can use to check Rhai scripts.
//!
//! Also includes action parameter schemas loaded from TOML.

use std::collections::HashMap;
use std::path::Path;
use serde::Deserialize;

// =============================================================================
// Archetype Schema (from derive macro)
// =============================================================================

/// Schema information for an archetype struct.
#[derive(Debug, Clone)]
pub struct ArchetypeSchema {
    /// Name of the struct
    pub name: &'static str,
    /// Field schemas
    pub fields: &'static [FieldSchema],
}

impl ArchetypeSchema {
    /// Get all required field names.
    pub fn required_fields(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().filter(|f| f.required).map(|f| f.name)
    }

    /// Get a field schema by name.
    pub fn get_field(&self, name: &str) -> Option<&FieldSchema> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Check if a field exists.
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name)
    }

    /// Get all field names.
    pub fn field_names(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().map(|f| f.name)
    }
}

/// Schema information for a single field.
#[derive(Debug, Clone, Copy)]
pub struct FieldSchema {
    /// Field name in Rhai (after rename if applicable)
    pub name: &'static str,
    /// Rust type name
    pub rust_type: &'static str,
    /// Whether the field is required (no default)
    pub required: bool,
}

impl FieldSchema {
    /// Check if this field's type is a string type.
    pub fn is_string(&self) -> bool {
        self.rust_type == "String" || self.rust_type.contains("String")
    }

    /// Check if this field's type is numeric.
    pub fn is_numeric(&self) -> bool {
        matches!(self.rust_type, "f32" | "f64" | "i32" | "i64" | "u32" | "u64" | "i16" | "u16")
    }

    /// Check if this field's type is a boolean.
    pub fn is_bool(&self) -> bool {
        self.rust_type == "bool"
    }

    /// Check if this field's type is an array/vec.
    pub fn is_array(&self) -> bool {
        self.rust_type.starts_with("Vec<")
    }

    /// Check if this field's type is optional.
    pub fn is_optional(&self) -> bool {
        self.rust_type.starts_with("Option<")
    }
}

/// Trait for types that provide schema information for validation.
pub trait RhaiSchema {
    /// Get the schema for this type.
    fn schema() -> ArchetypeSchema;
}

// =============================================================================
// Action Parameter Schema (from TOML)
// =============================================================================

/// Schema for a single action's parameters.
#[derive(Debug, Clone, Deserialize)]
pub struct ActionSchema {
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    /// Parameter definitions
    #[serde(default)]
    pub params: HashMap<String, ParamSchema>,
}

impl ActionSchema {
    /// Get all required parameter names.
    pub fn required_params(&self) -> impl Iterator<Item = &str> {
        self.params.iter()
            .filter(|(_, p)| p.required)
            .map(|(k, _)| k.as_str())
    }

    /// Check if a parameter exists.
    pub fn has_param(&self, name: &str) -> bool {
        self.params.contains_key(name)
    }

    /// Get a parameter schema by name.
    pub fn get_param(&self, name: &str) -> Option<&ParamSchema> {
        self.params.get(name)
    }
}

/// Schema for a single parameter.
#[derive(Debug, Clone, Deserialize)]
pub struct ParamSchema {
    /// Type of the parameter: uuid, string, int, float, bool, array
    #[serde(rename = "type")]
    pub param_type: String,
    /// Whether this parameter is required
    #[serde(default)]
    pub required: bool,
}

impl ParamSchema {
    pub fn is_uuid(&self) -> bool {
        self.param_type == "uuid"
    }

    pub fn is_string(&self) -> bool {
        self.param_type == "string"
    }

    pub fn is_int(&self) -> bool {
        self.param_type == "int"
    }

    pub fn is_float(&self) -> bool {
        self.param_type == "float"
    }

    pub fn is_bool(&self) -> bool {
        self.param_type == "bool"
    }

    pub fn is_array(&self) -> bool {
        self.param_type == "array"
    }
}

/// Collection of all action schemas.
#[derive(Debug, Clone, Default)]
pub struct ActionSchemaRegistry {
    /// Map from action name to schema
    pub actions: HashMap<String, ActionSchema>,
}

impl ActionSchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load action schemas from a TOML file.
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("Failed to read schema file: {}", e))?;
        Self::load_from_str(&content)
    }

    /// Load action schemas from a TOML string.
    pub fn load_from_str(content: &str) -> Result<Self, String> {
        let actions: HashMap<String, ActionSchema> = toml::from_str(content)
            .map_err(|e| format!("Failed to parse schema TOML: {}", e))?;
        Ok(Self { actions })
    }

    /// Get schema for an action.
    pub fn get(&self, action: &str) -> Option<&ActionSchema> {
        self.actions.get(action)
    }

    /// Check if an action is known.
    pub fn has_action(&self, action: &str) -> bool {
        self.actions.contains_key(action)
    }

    /// Get all known action names.
    pub fn action_names(&self) -> impl Iterator<Item = &str> {
        self.actions.keys().map(|s| s.as_str())
    }

    /// Validate that a params access is valid for an action.
    ///
    /// Returns an error message if the param is unknown, or None if valid.
    pub fn validate_param_access(&self, action: &str, param_key: &str) -> Option<String> {
        if let Some(schema) = self.get(action)
            && !schema.has_param(param_key) {
                return Some(format!(
                    "Unknown param '{}' for action '{}'. Valid params: {:?}",
                    param_key,
                    action,
                    schema.params.keys().collect::<Vec<_>>()
                ));
            }
        // If action is unknown, we don't validate (it might be a custom action)
        None
    }
}

// =============================================================================
// Definition Schema (for definition scripts like ships.rhai, weapons.rhai)
// =============================================================================

/// Expected type of a definition field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefFieldType {
    /// String value
    String,
    /// Numeric value (int or float)
    Number,
    /// Boolean value
    Bool,
    /// Array of any type
    Array,
    /// Map/Object
    Map,
    /// Any type (no validation)
    Any,
}

impl DefFieldType {
    /// Get display name for error messages.
    pub fn name(&self) -> &'static str {
        match self {
            DefFieldType::String => "string",
            DefFieldType::Number => "number",
            DefFieldType::Bool => "bool",
            DefFieldType::Array => "array",
            DefFieldType::Map => "map",
            DefFieldType::Any => "any",
        }
    }
}

/// Schema for a single field in a definition.
#[derive(Debug, Clone)]
pub struct DefFieldSchema {
    /// Field name
    pub name: &'static str,
    /// Expected type
    pub field_type: DefFieldType,
    /// Whether this field is required
    pub required: bool,
}

impl DefFieldSchema {
    /// Create a required field.
    pub const fn required(name: &'static str, field_type: DefFieldType) -> Self {
        Self { name, field_type, required: true }
    }

    /// Create an optional field.
    pub const fn optional(name: &'static str, field_type: DefFieldType) -> Self {
        Self { name, field_type, required: false }
    }
}

/// Schema for a definition type (e.g., ShipArchetype, WeaponArchetype).
#[derive(Debug, Clone)]
pub struct DefinitionSchema {
    /// Name of the archetype type
    pub name: &'static str,
    /// Field schemas
    pub fields: &'static [DefFieldSchema],
}

impl DefinitionSchema {
    /// Get all required field names.
    pub fn required_fields(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().filter(|f| f.required).map(|f| f.name)
    }

    /// Get a field schema by name.
    pub fn get_field(&self, name: &str) -> Option<&DefFieldSchema> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Check if a field exists.
    pub fn has_field(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name)
    }

    /// Get all field names.
    pub fn field_names(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().map(|f| f.name)
    }
}

/// Registry of definition schemas for validation.
#[derive(Debug, Clone, Default)]
pub struct DefinitionSchemaRegistry {
    schemas: HashMap<String, &'static DefinitionSchema>,
}

impl DefinitionSchemaRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry with all built-in schemas.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        registry.register("ship", &SHIP_SCHEMA);
        registry.register("weapon", &WEAPON_SCHEMA);
        registry.register("effect", &EFFECT_SCHEMA);
        registry.register("cargo", &CARGO_SCHEMA);
        registry.register("ability", &ABILITY_SCHEMA);
        registry.register("faction", &FACTION_SCHEMA);
        registry
    }

    /// Register a schema for a definition type.
    pub fn register(&mut self, name: &str, schema: &'static DefinitionSchema) {
        self.schemas.insert(name.to_string(), schema);
    }

    /// Get schema for a definition type.
    pub fn get(&self, name: &str) -> Option<&'static DefinitionSchema> {
        self.schemas.get(name).copied()
    }

    /// Infer definition type from script path.
    pub fn infer_type_from_path(&self, path: &str) -> Option<&str> {
        if path.contains("ships") {
            Some("ship")
        } else if path.contains("weapons") {
            Some("weapon")
        } else if path.contains("effects") {
            Some("effect")
        } else if path.contains("cargo") {
            Some("cargo")
        } else if path.contains("abilities") {
            Some("ability")
        } else if path.contains("factions") {
            Some("faction")
        } else {
            None
        }
    }
}

// =============================================================================
// Built-in Definition Schemas
// =============================================================================

/// Ship archetype fields.
pub static SHIP_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "ShipArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("tier", DefFieldType::Number),
        DefFieldSchema::optional("attack", DefFieldType::Number),
        DefFieldSchema::optional("defense", DefFieldType::Number),
        DefFieldSchema::optional("speed", DefFieldType::Number),
        DefFieldSchema::optional("shield_capacity", DefFieldType::Number),
        DefFieldSchema::optional("sensor_range", DefFieldType::Number),
        DefFieldSchema::optional("cargo_capacity", DefFieldType::Number),
        DefFieldSchema::optional("fuel_per_sector", DefFieldType::Number),
        DefFieldSchema::optional("ammo_per_attack", DefFieldType::Number),
        DefFieldSchema::optional("weapons", DefFieldType::Array),
        DefFieldSchema::optional("upgrade_slots", DefFieldType::Array),
        DefFieldSchema::optional("is_player_class", DefFieldType::Bool),
        DefFieldSchema::optional("is_hostile", DefFieldType::Bool),
        DefFieldSchema::optional("behaviors", DefFieldType::Array),
        DefFieldSchema::optional("unlock", DefFieldType::Map),
        // Nested stats fields (when not using nested stats object)
        DefFieldSchema::optional("stats", DefFieldType::Map),
    ],
};

/// Weapon archetype fields.
pub static WEAPON_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "WeaponArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("damage", DefFieldType::Number),
        DefFieldSchema::optional("accuracy", DefFieldType::Number),
        DefFieldSchema::optional("ammo_cost", DefFieldType::Number),
        DefFieldSchema::optional("range_modifier", DefFieldType::Number),
        DefFieldSchema::optional("effects", DefFieldType::Array),
        DefFieldSchema::optional("category", DefFieldType::String),
        DefFieldSchema::optional("tier", DefFieldType::Number),
        DefFieldSchema::optional("cost", DefFieldType::Number),
    ],
};

/// Effect archetype fields.
pub static EFFECT_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "EffectArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("icon", DefFieldType::String),
        DefFieldSchema::optional("type", DefFieldType::String),
        DefFieldSchema::optional("handler", DefFieldType::String),
        DefFieldSchema::optional("params", DefFieldType::Map),
        DefFieldSchema::optional("stacking", DefFieldType::String),
        DefFieldSchema::optional("trigger", DefFieldType::String),
    ],
};

/// Cargo archetype fields.
pub static CARGO_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "CargoArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("base_price", DefFieldType::Number),
        DefFieldSchema::optional("weight", DefFieldType::Number),
        DefFieldSchema::optional("legal", DefFieldType::Bool),
        DefFieldSchema::optional("volatility", DefFieldType::Number),
        DefFieldSchema::optional("contraband_penalty", DefFieldType::Number),
        DefFieldSchema::optional("category", DefFieldType::String),
        DefFieldSchema::optional("special_effects", DefFieldType::Array),
        DefFieldSchema::optional("icon", DefFieldType::String),
        DefFieldSchema::optional("tier", DefFieldType::Number),
        DefFieldSchema::optional("abundance", DefFieldType::Number),
        DefFieldSchema::optional("demand_modifiers", DefFieldType::Map),
    ],
};

/// Ability archetype fields.
pub static ABILITY_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "AbilityArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("icon", DefFieldType::String),
        DefFieldSchema::optional("type", DefFieldType::String),
        DefFieldSchema::optional("target", DefFieldType::String),
        DefFieldSchema::optional("cost", DefFieldType::Map),
        DefFieldSchema::optional("cooldown", DefFieldType::Number),
        DefFieldSchema::optional("effects", DefFieldType::Array),
        DefFieldSchema::optional("requirements", DefFieldType::Map),
        DefFieldSchema::optional("tier", DefFieldType::Number),
        DefFieldSchema::optional("is_passive", DefFieldType::Bool),
    ],
};

/// Faction archetype fields.
pub static FACTION_SCHEMA: DefinitionSchema = DefinitionSchema {
    name: "FactionArchetype",
    fields: &[
        DefFieldSchema::required("id", DefFieldType::String),
        DefFieldSchema::required("name", DefFieldType::String),
        DefFieldSchema::optional("description", DefFieldType::String),
        DefFieldSchema::optional("color", DefFieldType::String),
        DefFieldSchema::optional("icon", DefFieldType::String),
        DefFieldSchema::optional("tag", DefFieldType::String),
        DefFieldSchema::optional("tier", DefFieldType::Number),
        DefFieldSchema::optional("is_playable", DefFieldType::Bool),
        DefFieldSchema::optional("is_hostile", DefFieldType::Bool),
        DefFieldSchema::optional("is_territorial", DefFieldType::Bool),
        DefFieldSchema::optional("default_standing", DefFieldType::Number),
        DefFieldSchema::optional("behaviors", DefFieldType::Map),
        DefFieldSchema::optional("behavior_modifiers", DefFieldType::Map),
        DefFieldSchema::optional("combat_bonuses", DefFieldType::Map),
        DefFieldSchema::optional("standing_requirements", DefFieldType::Map),
        DefFieldSchema::optional("relations", DefFieldType::Map),
        DefFieldSchema::optional("home_sectors", DefFieldType::Array),
        DefFieldSchema::optional("preferred_ships", DefFieldType::Array),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archetype_schema() {
        static FIELDS: [FieldSchema; 3] = [
            FieldSchema { name: "id", rust_type: "String", required: true },
            FieldSchema { name: "tier", rust_type: "u32", required: false },
            FieldSchema { name: "tags", rust_type: "Vec<String>", required: false },
        ];

        let schema = ArchetypeSchema {
            name: "TestArchetype",
            fields: &FIELDS,
        };

        assert_eq!(schema.name, "TestArchetype");
        assert_eq!(schema.fields.len(), 3);

        // Test required_fields
        let required: Vec<_> = schema.required_fields().collect();
        assert_eq!(required, vec!["id"]);

        // Test get_field
        let id_field = schema.get_field("id").unwrap();
        assert!(id_field.is_string());
        assert!(id_field.required);

        let tier_field = schema.get_field("tier").unwrap();
        assert!(tier_field.is_numeric());
        assert!(!tier_field.required);

        let tags_field = schema.get_field("tags").unwrap();
        assert!(tags_field.is_array());

        // Test has_field
        assert!(schema.has_field("id"));
        assert!(!schema.has_field("nonexistent"));
    }

    #[test]
    fn test_field_schema_type_checks() {
        let string_field = FieldSchema { name: "name", rust_type: "String", required: true };
        assert!(string_field.is_string());
        assert!(!string_field.is_numeric());

        let int_field = FieldSchema { name: "count", rust_type: "i32", required: true };
        assert!(int_field.is_numeric());
        assert!(!int_field.is_string());

        let bool_field = FieldSchema { name: "active", rust_type: "bool", required: true };
        assert!(bool_field.is_bool());

        let vec_field = FieldSchema { name: "items", rust_type: "Vec<String>", required: false };
        assert!(vec_field.is_array());

        let opt_field = FieldSchema { name: "desc", rust_type: "Option<String>", required: false };
        assert!(opt_field.is_optional());
    }

    // ========================================================================
    // Action Schema Tests
    // ========================================================================

    #[test]
    fn test_action_schema_load() {
        let toml = r#"
            [dock]
            description = "Dock at a station"
            [dock.params]
            station_id = { type = "uuid", required = true }

            [undock]
            description = "Undock from current station"
            [undock.params]

            [fire_weapon]
            description = "Fire a weapon"
            [fire_weapon.params]
            weapon_index = { type = "int", required = true }
            target_id = { type = "uuid", required = true }
        "#;

        let registry = ActionSchemaRegistry::load_from_str(toml).unwrap();

        // Check dock action
        assert!(registry.has_action("dock"));
        let dock = registry.get("dock").unwrap();
        assert_eq!(dock.description, "Dock at a station");
        assert!(dock.has_param("station_id"));
        let station_id = dock.get_param("station_id").unwrap();
        assert!(station_id.is_uuid());
        assert!(station_id.required);

        // Check undock has no params
        let undock = registry.get("undock").unwrap();
        assert!(undock.params.is_empty());

        // Check fire_weapon
        let fire = registry.get("fire_weapon").unwrap();
        let required: Vec<_> = fire.required_params().collect();
        assert_eq!(required.len(), 2);
        assert!(required.contains(&"weapon_index"));
        assert!(required.contains(&"target_id"));

        // Check unknown action
        assert!(!registry.has_action("nonexistent"));
    }

    #[test]
    fn test_action_schema_validation() {
        let toml = r#"
            [dock]
            [dock.params]
            station_id = { type = "uuid", required = true }
        "#;

        let registry = ActionSchemaRegistry::load_from_str(toml).unwrap();

        // Valid param should return None
        assert!(registry.validate_param_access("dock", "station_id").is_none());

        // Invalid param should return error
        let error = registry.validate_param_access("dock", "invalid_key");
        assert!(error.is_some());
        assert!(error.unwrap().contains("Unknown param"));

        // Unknown action should not error (might be custom)
        assert!(registry.validate_param_access("custom_action", "any_param").is_none());
    }

    #[test]
    fn test_load_actual_schema_file() {
        // Get the actual schema file
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let schema_path = std::path::PathBuf::from(manifest_dir)
            .parent().unwrap()
            .parent().unwrap()
            .join("scripts")
            .join("schemas")
            .join("actions.toml");

        if schema_path.exists() {
            let registry = ActionSchemaRegistry::load_from_file(&schema_path)
                .expect("Should load schema file");

            // Check some expected actions exist
            assert!(registry.has_action("dock"), "Should have dock action");
            assert!(registry.has_action("undock"), "Should have undock action");
            assert!(registry.has_action("fire_weapon"), "Should have fire_weapon action");
            assert!(registry.has_action("engage_target"), "Should have engage_target action");

            // Check dock params
            let dock = registry.get("dock").unwrap();
            assert!(dock.has_param("station_id"));

            // Check fire_weapon params
            let fire = registry.get("fire_weapon").unwrap();
            assert!(fire.has_param("weapon_index"));
            assert!(fire.has_param("target_id"));

            eprintln!("Loaded {} action schemas from {:?}", registry.actions.len(), schema_path);
        } else {
            eprintln!("Schema file not found at {:?}, skipping", schema_path);
        }
    }
}
