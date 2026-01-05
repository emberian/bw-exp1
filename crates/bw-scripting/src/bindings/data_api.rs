//! Data API bindings for Rhai
//!
//! Provides access to game data files (TOML) and archetype definitions from scripts.
//! Data is loaded once and cached for fast access.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use rhai::{Engine, Dynamic, Map};

use bw_game::archetypes::ArchetypeRegistry;

// Thread-local data accessor for script execution.
thread_local! {
    static CURRENT_DATA: RefCell<Option<Arc<DataStore>>> = const { RefCell::new(None) };
    static CURRENT_ARCHETYPES: RefCell<Option<Arc<ArchetypeRegistry>>> = const { RefCell::new(None) };
}

/// Set the data store for the current thread during script execution.
pub fn set_current_data(data: Arc<DataStore>) {
    CURRENT_DATA.with(|cell| {
        *cell.borrow_mut() = Some(data);
    });
}

/// Clear the data store after script execution.
pub fn clear_current_data() {
    CURRENT_DATA.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Set the archetype registry for the current thread during script execution.
pub fn set_current_archetypes(registry: Arc<ArchetypeRegistry>) {
    CURRENT_ARCHETYPES.with(|cell| {
        *cell.borrow_mut() = Some(registry);
    });
}

/// Clear the archetype registry after script execution.
pub fn clear_current_archetypes() {
    CURRENT_ARCHETYPES.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Get data from the current store.
fn with_data<T, F: FnOnce(&DataStore) -> T>(f: F) -> Option<T> {
    CURRENT_DATA.with(|cell| {
        cell.borrow().as_ref().map(|data| f(data))
    })
}

/// Get archetypes from the current registry.
fn with_archetypes<T, F: FnOnce(&ArchetypeRegistry) -> T>(f: F) -> Option<T> {
    CURRENT_ARCHETYPES.with(|cell| {
        cell.borrow().as_ref().map(|registry| f(registry))
    })
}

/// Store for game data loaded from TOML files.
pub struct DataStore {
    /// Data indexed by category (file name without extension) and key
    data: RwLock<HashMap<String, HashMap<String, Dynamic>>>,
    /// Path to data directory
    data_dir: PathBuf,
}

impl DataStore {
    /// Create a new data store and load data from the directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let data_dir = data_dir.into();
        let store = Self {
            data: RwLock::new(HashMap::new()),
            data_dir,
        };
        store.load_all();
        store
    }

    /// Load all TOML files from the data directory.
    pub fn load_all(&self) {
        let dir = &self.data_dir;

        if !dir.exists() {
            tracing::warn!("Data directory {:?} does not exist", dir);
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::error!("Failed to read data directory {:?}: {}", dir, e);
                return;
            }
        };

        let mut data = self.data.write();
        data.clear();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") {
                if let Some(category) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_file(&path) {
                        Ok(entries) => {
                            tracing::info!("Loaded {} entries from data/{}.toml", entries.len(), category);
                            data.insert(category.to_string(), entries);
                        }
                        Err(e) => {
                            tracing::error!("Failed to load data file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    /// Load a single TOML file and convert to Dynamic maps.
    fn load_file(&self, path: &Path) -> anyhow::Result<HashMap<String, Dynamic>> {
        let contents = std::fs::read_to_string(path)?;
        let table: toml::Table = toml::from_str(&contents)?;

        let mut result = HashMap::new();
        for (key, value) in table {
            result.insert(key, toml_to_dynamic(value));
        }

        Ok(result)
    }

    /// Reload all data files.
    pub fn reload(&self) {
        tracing::info!("Reloading data files from {:?}", self.data_dir);
        self.load_all();
    }

    /// Get data by category and key.
    pub fn get(&self, category: &str, key: &str) -> Option<Dynamic> {
        let data = self.data.read();
        data.get(category)
            .and_then(|entries| entries.get(key))
            .cloned()
    }

    /// Get all entries in a category.
    pub fn get_category(&self, category: &str) -> Option<HashMap<String, Dynamic>> {
        let data = self.data.read();
        data.get(category).cloned()
    }

    /// List all keys in a category.
    pub fn list_keys(&self, category: &str) -> Vec<String> {
        let data = self.data.read();
        data.get(category)
            .map(|entries| entries.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// Convert a TOML value to a Rhai Dynamic.
fn toml_to_dynamic(value: toml::Value) -> Dynamic {
    match value {
        toml::Value::String(s) => Dynamic::from(s),
        toml::Value::Integer(i) => Dynamic::from(i),
        toml::Value::Float(f) => Dynamic::from(f),
        toml::Value::Boolean(b) => Dynamic::from(b),
        toml::Value::Datetime(dt) => Dynamic::from(dt.to_string()),
        toml::Value::Array(arr) => {
            let arr: rhai::Array = arr.into_iter().map(toml_to_dynamic).collect();
            Dynamic::from(arr)
        }
        toml::Value::Table(table) => {
            let mut map = Map::new();
            for (k, v) in table {
                map.insert(k.into(), toml_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

/// Register data API functions with the engine.
pub fn register(engine: &mut Engine) {
    // get_data(category: String, key: String) -> Map
    // Returns data entry or empty map if not found
    engine.register_fn("get_data", |category: String, key: String| -> Dynamic {
        with_data(|data| {
            data.get(&category, &key).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_ship_def(ship_class: String) -> Map
    // Convenience function for ship data
    engine.register_fn("get_ship_def", |ship_class: String| -> Dynamic {
        with_data(|data| {
            data.get("ships", &ship_class).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_upgrade_def(upgrade_id: String) -> Map
    // Convenience function for upgrade data
    engine.register_fn("get_upgrade_def", |upgrade_id: String| -> Dynamic {
        with_data(|data| {
            data.get("upgrades", &upgrade_id).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_cargo_def(cargo_type: String) -> Map
    // Convenience function for cargo data
    // Now tries archetype registry first, falls back to TOML
    engine.register_fn("get_cargo_def", |cargo_type: String| -> Dynamic {
        // Try archetype registry first
        let archetype_result = with_archetypes(|registry| {
            registry.get_cargo(&cargo_type)
                .map(|arch| cargo_to_dynamic(&arch))
        }).flatten();

        if let Some(result) = archetype_result {
            return result;
        }

        // Fallback to TOML data
        with_data(|data| {
            data.get("cargo", &cargo_type).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // get_stance_def(stance: String) -> Map
    // Convenience function for combat stance data
    engine.register_fn("get_stance_def", |stance: String| -> Dynamic {
        with_data(|data| {
            data.get("combat_stances", &stance).unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    // list_data_keys(category: String) -> Array
    // Returns array of keys in a data category
    engine.register_fn("list_data_keys", |category: String| -> rhai::Array {
        with_data(|data| {
            let keys = data.list_keys(&category);
            keys.into_iter().map(Dynamic::from).collect()
        }).unwrap_or_default()
    });

    // has_data(category: String, key: String) -> bool
    // Check if data entry exists
    engine.register_fn("has_data", |category: String, key: String| -> bool {
        with_data(|data| {
            data.get(&category, &key).is_some()
        }).unwrap_or(false)
    });

    // =========================================================================
    // Archetype Registry Functions
    // =========================================================================

    // --- Ship Archetypes ---
    engine.register_fn("get_ship", |id: String| -> Dynamic {
        with_archetypes(|registry| {
            registry.get_ship(&id)
                .map(|arch| ship_to_dynamic(&arch))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("all_ships", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_ships()
                .iter()
                .map(|arch| ship_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("player_ships", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.player_ships()
                .iter()
                .map(|arch| ship_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    // --- Weapon Archetypes ---
    engine.register_fn("get_weapon", |id: String| -> Dynamic {
        with_archetypes(|registry| {
            registry.get_weapon(&id)
                .map(|arch| weapon_to_dynamic(&arch))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("all_weapons", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_weapons()
                .iter()
                .map(|arch| weapon_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    // --- Effect Archetypes ---
    engine.register_fn("get_effect", |id: String| -> Dynamic {
        with_archetypes(|registry| {
            registry.get_effect(&id)
                .map(|arch| effect_to_dynamic(&arch))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("all_effects", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_effects()
                .iter()
                .map(|arch| effect_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    // --- Ability Archetypes ---
    engine.register_fn("get_ability", |id: String| -> Dynamic {
        with_archetypes(|registry| {
            registry.get_ability(&id)
                .map(|arch| ability_to_dynamic(&arch))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("all_abilities", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_abilities()
                .iter()
                .map(|arch| ability_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("active_abilities", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.active_abilities()
                .iter()
                .map(|arch| ability_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("passive_abilities", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.passive_abilities()
                .iter()
                .map(|arch| ability_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    // --- Cargo Archetypes ---
    // Note: get_cargo_def is updated to use archetypes first, fallback to TOML
    // The name "get_cargo_def" is kept for backward compatibility with existing scripts
    // (get_cargo is already used by state_api for ship cargo inventory)

    engine.register_fn("all_cargo_types", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_cargo()
                .iter()
                .map(|arch| cargo_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("legal_cargo_types", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.legal_cargo()
                .iter()
                .map(|arch| cargo_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    // --- Faction Archetypes ---
    engine.register_fn("get_faction", |id: String| -> Dynamic {
        with_archetypes(|registry| {
            registry.get_faction(&id)
                .map(|arch| faction_to_dynamic(&arch))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    });

    engine.register_fn("all_factions", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.all_factions()
                .iter()
                .map(|arch| faction_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("playable_factions", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.playable_factions()
                .iter()
                .map(|arch| faction_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("hostile_factions", || -> rhai::Array {
        with_archetypes(|registry| {
            registry.hostile_factions()
                .iter()
                .map(|arch| faction_to_dynamic(arch))
                .collect()
        }).unwrap_or_default()
    });

    engine.register_fn("get_faction_relation", |faction_a: String, faction_b: String| -> i64 {
        with_archetypes(|registry| {
            registry.get_faction_relation(&faction_a, &faction_b) as i64
        }).unwrap_or(0)
    });
}

// =========================================================================
// Archetype to Dynamic Converters
// =========================================================================

use bw_game::archetypes::{
    ShipArchetype, WeaponArchetype, EffectArchetype, AbilityArchetype,
    CargoArchetype, FactionArchetype,
};

fn ship_to_dynamic(arch: &ShipArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("tier".into(), Dynamic::from(arch.tier as i64));
    map.insert("is_player_class".into(), Dynamic::from(arch.is_player_class));
    map.insert("is_hostile".into(), Dynamic::from(arch.is_hostile));

    // Stats
    let mut stats = Map::new();
    stats.insert("attack".into(), Dynamic::from(arch.stats.attack as f64));
    stats.insert("defense".into(), Dynamic::from(arch.stats.defense as f64));
    stats.insert("speed".into(), Dynamic::from(arch.stats.speed as f64));
    stats.insert("shield_capacity".into(), Dynamic::from(arch.stats.shield_capacity as f64));
    stats.insert("sensor_range".into(), Dynamic::from(arch.stats.sensor_range as f64));
    stats.insert("cargo_capacity".into(), Dynamic::from(arch.stats.cargo_capacity as i64));
    stats.insert("fuel_per_sector".into(), Dynamic::from(arch.stats.fuel_per_sector as f64));
    stats.insert("ammo_per_attack".into(), Dynamic::from(arch.stats.ammo_per_attack as f64));
    map.insert("stats".into(), Dynamic::from(stats));

    // Weapons
    let weapons: rhai::Array = arch.weapons.iter().map(|w| {
        let mut wmap = Map::new();
        wmap.insert("type".into(), Dynamic::from(w.weapon_type.clone()));
        wmap.insert("damage".into(), Dynamic::from(w.damage as f64));
        wmap.insert("accuracy".into(), Dynamic::from(w.accuracy as f64));
        wmap.insert("ammo_cost".into(), Dynamic::from(w.ammo_cost as f64));
        Dynamic::from(wmap)
    }).collect();
    map.insert("weapons".into(), Dynamic::from(weapons));

    // Behaviors
    let behaviors: rhai::Array = arch.behaviors.iter()
        .map(|b| Dynamic::from(b.clone()))
        .collect();
    map.insert("behaviors".into(), Dynamic::from(behaviors));

    Dynamic::from(map)
}

fn weapon_to_dynamic(arch: &WeaponArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("damage".into(), Dynamic::from(arch.damage as f64));
    map.insert("accuracy".into(), Dynamic::from(arch.accuracy as f64));
    map.insert("ammo_cost".into(), Dynamic::from(arch.ammo_cost as f64));
    map.insert("range_modifier".into(), Dynamic::from(arch.range_modifier as f64));
    map.insert("category".into(), Dynamic::from(arch.category.clone()));
    map.insert("tier".into(), Dynamic::from(arch.tier as i64));
    map.insert("cost".into(), Dynamic::from(arch.cost));

    let effects: rhai::Array = arch.effects.iter()
        .map(|e| Dynamic::from(e.clone()))
        .collect();
    map.insert("effects".into(), Dynamic::from(effects));

    Dynamic::from(map)
}

fn effect_to_dynamic(arch: &EffectArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("effect_type".into(), Dynamic::from(format!("{:?}", arch.effect_type)));
    map.insert("handler".into(), Dynamic::from(arch.handler.clone()));
    map.insert("stacking".into(), Dynamic::from(format!("{:?}", arch.stacking)));
    if let Some(ref trigger) = arch.trigger {
        map.insert("trigger".into(), Dynamic::from(format!("{:?}", trigger)));
    }

    // Convert params map
    let params: Map = arch.params.iter()
        .map(|(k, v)| (k.clone().into(), v.clone()))
        .collect();
    map.insert("params".into(), Dynamic::from(params));

    Dynamic::from(map)
}

fn ability_to_dynamic(arch: &AbilityArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("cooldown".into(), Dynamic::from(arch.cooldown as i64));
    map.insert("target".into(), Dynamic::from(format!("{:?}", arch.target)));
    map.insert("tier".into(), Dynamic::from(arch.tier as i64));
    map.insert("is_passive".into(), Dynamic::from(arch.is_passive));

    // Cost
    let mut cost = Map::new();
    if arch.cost.energy > 0 {
        cost.insert("energy".into(), Dynamic::from(arch.cost.energy as i64));
    }
    if arch.cost.fuel > 0 {
        cost.insert("fuel".into(), Dynamic::from(arch.cost.fuel as i64));
    }
    if arch.cost.ammunition > 0 {
        cost.insert("ammunition".into(), Dynamic::from(arch.cost.ammunition as i64));
    }
    if arch.cost.credits > 0 {
        cost.insert("credits".into(), Dynamic::from(arch.cost.credits));
    }
    if arch.cost.shields > 0 {
        cost.insert("shields".into(), Dynamic::from(arch.cost.shields as i64));
    }
    map.insert("cost".into(), Dynamic::from(cost));

    // Effects
    let effects: rhai::Array = arch.effects.iter().map(|e| {
        let mut emap = Map::new();
        emap.insert("effect_id".into(), Dynamic::from(e.effect_id.clone()));
        let params: Map = e.params.iter()
            .map(|(k, v)| (k.clone().into(), v.clone()))
            .collect();
        emap.insert("params".into(), Dynamic::from(params));
        Dynamic::from(emap)
    }).collect();
    map.insert("effects".into(), Dynamic::from(effects));

    Dynamic::from(map)
}

fn cargo_to_dynamic(arch: &CargoArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("base_price".into(), Dynamic::from(arch.base_price));
    map.insert("weight".into(), Dynamic::from(arch.weight as i64));
    map.insert("legal".into(), Dynamic::from(arch.legal));
    map.insert("volatility".into(), Dynamic::from(arch.volatility as f64));
    map.insert("contraband_penalty".into(), Dynamic::from(arch.contraband_penalty as i64));
    map.insert("category".into(), Dynamic::from(format!("{:?}", arch.category)));
    map.insert("tier".into(), Dynamic::from(arch.tier as i64));
    map.insert("abundance".into(), Dynamic::from(arch.abundance as f64));

    let effects: rhai::Array = arch.special_effects.iter()
        .map(|e| Dynamic::from(e.clone()))
        .collect();
    map.insert("special_effects".into(), Dynamic::from(effects));

    Dynamic::from(map)
}

fn faction_to_dynamic(arch: &FactionArchetype) -> Dynamic {
    let mut map = Map::new();
    map.insert("id".into(), Dynamic::from(arch.id.clone()));
    map.insert("name".into(), Dynamic::from(arch.name.clone()));
    map.insert("tag".into(), Dynamic::from(arch.tag.clone()));
    if let Some(ref desc) = arch.description {
        map.insert("description".into(), Dynamic::from(desc.clone()));
    }
    map.insert("color".into(), Dynamic::from(arch.color.clone()));
    map.insert("is_playable".into(), Dynamic::from(arch.is_playable));
    map.insert("is_hostile".into(), Dynamic::from(arch.is_hostile));
    map.insert("is_territorial".into(), Dynamic::from(arch.is_territorial));
    map.insert("tier".into(), Dynamic::from(arch.tier as i64));

    // Behavior modifiers
    let mut behaviors = Map::new();
    behaviors.insert("aggression".into(), Dynamic::from(arch.behavior_modifiers.aggression as f64));
    behaviors.insert("trade_preference".into(), Dynamic::from(arch.behavior_modifiers.trade_preference as f64));
    behaviors.insert("patrol_range".into(), Dynamic::from(arch.behavior_modifiers.patrol_range as f64));
    behaviors.insert("flee_threshold".into(), Dynamic::from(arch.behavior_modifiers.flee_threshold as f64));
    if let Some(ref pref) = arch.behavior_modifiers.target_preference {
        behaviors.insert("target_preference".into(), Dynamic::from(pref.clone()));
    }
    behaviors.insert("calls_reinforcements".into(), Dynamic::from(arch.behavior_modifiers.calls_reinforcements));
    behaviors.insert("surrender_chance".into(), Dynamic::from(arch.behavior_modifiers.surrender_chance as f64));
    map.insert("behavior_modifiers".into(), Dynamic::from(behaviors));

    // Combat bonuses
    let mut bonuses = Map::new();
    bonuses.insert("attack_bonus".into(), Dynamic::from(arch.combat_bonuses.attack_bonus as f64));
    bonuses.insert("defense_bonus".into(), Dynamic::from(arch.combat_bonuses.defense_bonus as f64));
    bonuses.insert("speed_bonus".into(), Dynamic::from(arch.combat_bonuses.speed_bonus as f64));
    bonuses.insert("accuracy_bonus".into(), Dynamic::from(arch.combat_bonuses.accuracy_bonus as f64));
    bonuses.insert("shield_bonus".into(), Dynamic::from(arch.combat_bonuses.shield_bonus as f64));
    bonuses.insert("critical_bonus".into(), Dynamic::from(arch.combat_bonuses.critical_bonus as f64));
    map.insert("combat_bonuses".into(), Dynamic::from(bonuses));

    // Relations
    let relations: Map = arch.relations.iter()
        .map(|(k, v)| (k.clone().into(), Dynamic::from(*v as i64)))
        .collect();
    map.insert("relations".into(), Dynamic::from(relations));

    // Home sectors and preferred ships
    let home_sectors: rhai::Array = arch.home_sectors.iter()
        .map(|s| Dynamic::from(s.clone()))
        .collect();
    map.insert("home_sectors".into(), Dynamic::from(home_sectors));

    let preferred_ships: rhai::Array = arch.preferred_ships.iter()
        .map(|s| Dynamic::from(s.clone()))
        .collect();
    map.insert("preferred_ships".into(), Dynamic::from(preferred_ships));

    Dynamic::from(map)
}
