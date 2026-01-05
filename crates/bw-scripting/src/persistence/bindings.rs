//! Rhai Bindings for Script Persistence
//!
//! Provides script functions for saving and loading persistent state.
//! Uses the unified `ScriptExecutionContext` for accessing the store.
//!
//! # Example
//!
//! ```rhai
//! // Save state with a namespaced key
//! save_state("nemesis:player123", #{ name: "Dread Pirate", defeats: 3 });
//!
//! // Load state later
//! let history = load_state("nemesis:player123");
//! if history != () {
//!     print(`${history.name} has been defeated ${history.defeats} times`);
//! }
//!
//! // Delete state
//! delete_state("nemesis:player123");
//!
//! // List keys in a namespace
//! let keys = list_state_keys("nemesis");
//!
//! // Create/restore checkpoints
//! checkpoint("before_boss");
//! // ... do stuff ...
//! restore_checkpoint("before_boss");
//! ```

use std::sync::Arc;
use rhai::{Engine, Dynamic, Map};

use super::store::ScriptStateStore;
use crate::context::with_context;

/// Get the current state store from the unified execution context.
fn get_store() -> Option<Arc<dyn ScriptStateStore>> {
    with_context(|ctx| ctx.persistence_store.clone()).flatten()
}

/// Parse a key into namespace and key parts.
/// Format: "namespace:key" or just "key" (uses "default" namespace)
fn parse_key(full_key: &str) -> (String, String) {
    if let Some((namespace, key)) = full_key.split_once(':') {
        (namespace.to_string(), key.to_string())
    } else {
        ("default".to_string(), full_key.to_string())
    }
}

/// Register persistence bindings with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // save_state("namespace:key", value)
    engine.register_fn("save_state", |key: &str, value: Dynamic| -> bool {
        let Some(store) = get_store() else {
            tracing::warn!("save_state called but no store is set");
            return false;
        };

        let (namespace, key) = parse_key(key);

        // Serialize Dynamic to JSON bytes
        let json = match serde_json::to_vec(&dynamic_to_json(&value)) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize state: {}", e);
                return false;
            }
        };

        match store.save(&namespace, &key, &json) {
            Ok(()) => true,
            Err(e) => {
                tracing::error!("Failed to save state: {}", e);
                false
            }
        }
    });

    // load_state("namespace:key") -> value or ()
    engine.register_fn("load_state", |key: &str| -> Dynamic {
        let Some(store) = get_store() else {
            tracing::warn!("load_state called but no store is set");
            return Dynamic::UNIT;
        };

        let (namespace, key) = parse_key(key);

        match store.load(&namespace, &key) {
            Ok(Some(data)) => {
                match serde_json::from_slice::<serde_json::Value>(&data) {
                    Ok(json) => json_to_dynamic(&json),
                    Err(e) => {
                        tracing::error!("Failed to deserialize state: {}", e);
                        Dynamic::UNIT
                    }
                }
            }
            Ok(None) => Dynamic::UNIT,
            Err(e) => {
                tracing::error!("Failed to load state: {}", e);
                Dynamic::UNIT
            }
        }
    });

    // delete_state("namespace:key") -> bool
    engine.register_fn("delete_state", |key: &str| -> bool {
        let Some(store) = get_store() else {
            tracing::warn!("delete_state called but no store is set");
            return false;
        };

        let (namespace, key) = parse_key(key);

        match store.delete(&namespace, &key) {
            Ok(deleted) => deleted,
            Err(e) => {
                tracing::error!("Failed to delete state: {}", e);
                false
            }
        }
    });

    // state_exists("namespace:key") -> bool
    engine.register_fn("state_exists", |key: &str| -> bool {
        let Some(store) = get_store() else {
            return false;
        };

        let (namespace, key) = parse_key(key);

        store.exists(&namespace, &key).unwrap_or(false)
    });

    // list_state_keys("namespace") -> [keys]
    engine.register_fn("list_state_keys", |namespace: &str| -> rhai::Array {
        let Some(store) = get_store() else {
            tracing::warn!("list_state_keys called but no store is set");
            return rhai::Array::new();
        };

        match store.list_keys(namespace) {
            Ok(keys) => keys.into_iter().map(Dynamic::from).collect(),
            Err(e) => {
                tracing::error!("Failed to list state keys: {}", e);
                rhai::Array::new()
            }
        }
    });

    // clear_state_namespace("namespace") -> count
    engine.register_fn("clear_state_namespace", |namespace: &str| -> i64 {
        let Some(store) = get_store() else {
            tracing::warn!("clear_state_namespace called but no store is set");
            return 0;
        };

        match store.clear_namespace(namespace) {
            Ok(count) => count as i64,
            Err(e) => {
                tracing::error!("Failed to clear namespace: {}", e);
                0
            }
        }
    });

    // checkpoint("name") -> bool
    engine.register_fn("checkpoint", |name: &str| -> bool {
        let Some(store) = get_store() else {
            tracing::warn!("checkpoint called but no store is set");
            return false;
        };

        match store.checkpoint(name) {
            Ok(()) => {
                tracing::info!(checkpoint = name, "Checkpoint created");
                true
            }
            Err(e) => {
                tracing::error!("Failed to create checkpoint: {}", e);
                false
            }
        }
    });

    // restore_checkpoint("name") -> bool
    engine.register_fn("restore_checkpoint", |name: &str| -> bool {
        let Some(store) = get_store() else {
            tracing::warn!("restore_checkpoint called but no store is set");
            return false;
        };

        match store.restore_checkpoint(name) {
            Ok(restored) => {
                if restored {
                    tracing::info!(checkpoint = name, "Checkpoint restored");
                }
                restored
            }
            Err(e) => {
                tracing::error!("Failed to restore checkpoint: {}", e);
                false
            }
        }
    });

    // list_checkpoints() -> [names]
    engine.register_fn("list_checkpoints", || -> rhai::Array {
        let Some(store) = get_store() else {
            return rhai::Array::new();
        };

        match store.list_checkpoints() {
            Ok(names) => names.into_iter().map(Dynamic::from).collect(),
            Err(e) => {
                tracing::error!("Failed to list checkpoints: {}", e);
                rhai::Array::new()
            }
        }
    });

    // delete_checkpoint("name") -> bool
    engine.register_fn("delete_checkpoint", |name: &str| -> bool {
        let Some(store) = get_store() else {
            return false;
        };

        match store.delete_checkpoint(name) {
            Ok(deleted) => deleted,
            Err(e) => {
                tracing::error!("Failed to delete checkpoint: {}", e);
                false
            }
        }
    });

    // get_state_info("namespace:key") -> #{ created_at, updated_at, version } or ()
    engine.register_fn("get_state_info", |key: &str| -> Dynamic {
        let Some(store) = get_store() else {
            return Dynamic::UNIT;
        };

        let (namespace, key) = parse_key(key);

        match store.get_metadata(&namespace, &key) {
            Ok(Some(state)) => {
                let mut map = Map::new();
                map.insert("namespace".into(), state.namespace.into());
                map.insert("key".into(), state.key.into());
                map.insert("created_at".into(), (state.created_at as i64).into());
                map.insert("updated_at".into(), (state.updated_at as i64).into());
                map.insert("version".into(), (state.version as i64).into());
                Dynamic::from(map)
            }
            Ok(None) => Dynamic::UNIT,
            Err(e) => {
                tracing::error!("Failed to get state info: {}", e);
                Dynamic::UNIT
            }
        }
    });
}

/// Convert a Rhai Dynamic to JSON Value for serialization.
fn dynamic_to_json(value: &Dynamic) -> serde_json::Value {
    if value.is_unit() {
        serde_json::Value::Null
    } else if let Some(b) = value.clone().try_cast::<bool>() {
        serde_json::Value::Bool(b)
    } else if let Some(i) = value.clone().try_cast::<i64>() {
        serde_json::Value::Number(i.into())
    } else if let Some(f) = value.clone().try_cast::<f64>() {
        // Handle special float values that JSON doesn't support natively
        if f.is_nan() {
            serde_json::Value::String("__NaN__".to_string())
        } else if f.is_infinite() {
            if f.is_sign_positive() {
                serde_json::Value::String("__Infinity__".to_string())
            } else {
                serde_json::Value::String("__-Infinity__".to_string())
            }
        } else {
            serde_json::json!(f)
        }
    } else if let Some(s) = value.clone().try_cast::<String>() {
        serde_json::Value::String(s)
    } else if let Some(arr) = value.clone().try_cast::<rhai::Array>() {
        serde_json::Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if let Some(map) = value.clone().try_cast::<Map>() {
        let obj: serde_json::Map<String, serde_json::Value> = map
            .iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_json(v)))
            .collect();
        serde_json::Value::Object(obj)
    } else {
        // Fallback: try to convert to string
        serde_json::Value::String(value.to_string())
    }
}

/// Convert a JSON Value to Rhai Dynamic.
fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => {
            // Handle special float values encoded as strings
            match s.as_str() {
                "__NaN__" => Dynamic::from(f64::NAN),
                "__Infinity__" => Dynamic::from(f64::INFINITY),
                "__-Infinity__" => Dynamic::from(f64::NEG_INFINITY),
                _ => Dynamic::from(s.clone()),
            }
        }
        serde_json::Value::Array(arr) => {
            let rhai_arr: rhai::Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(rhai_arr)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use crate::persistence::InMemoryStore;
    use crate::context::{ScriptExecutionContext, ExecutionGuard};
    use bw_game::state::{StateAccessor, StateProvider, ShipSnapshot, PlayerSnapshot, SectorSnapshot, StateMutation, MutationResult};
    use bw_core::models::Position;

    // Mock provider for tests
    struct MockProvider;
    impl StateProvider for MockProvider {
        fn get_ship(&self, _: Uuid) -> Option<ShipSnapshot> { None }
        fn get_ships_in_sector(&self, _: Uuid) -> Vec<ShipSnapshot> { vec![] }
        fn get_ships_in_range(&self, _: Uuid, _: Position, _: f64) -> Vec<ShipSnapshot> { vec![] }
        fn get_player(&self, _: Uuid) -> Option<PlayerSnapshot> { None }
        fn get_sector(&self, _: Uuid) -> Option<SectorSnapshot> { None }
        fn apply_mutations(&self, m: Vec<StateMutation>) -> Vec<MutationResult> {
            m.into_iter().map(MutationResult::success).collect()
        }
    }

    fn setup_test_context() -> (Arc<StateAccessor>, Arc<dyn ScriptStateStore>, ExecutionGuard) {
        let accessor = Arc::new(StateAccessor::new(Arc::new(MockProvider)));
        let store: Arc<dyn ScriptStateStore> = Arc::new(InMemoryStore::new());
        let ctx = ScriptExecutionContext::new(accessor.clone())
            .with_persistence_store(store.clone());
        let guard = ExecutionGuard::enter(ctx).expect("Failed to enter execution context");
        (accessor, store, guard)
    }

    #[test]
    fn test_parse_key() {
        assert_eq!(parse_key("nemesis:player123"), ("nemesis".to_string(), "player123".to_string()));
        assert_eq!(parse_key("simple_key"), ("default".to_string(), "simple_key".to_string()));
        assert_eq!(parse_key("a:b:c"), ("a".to_string(), "b:c".to_string()));
    }

    #[test]
    fn test_dynamic_json_roundtrip() {
        // Map
        let mut map = Map::new();
        map.insert("name".into(), Dynamic::from("test"));
        map.insert("count".into(), Dynamic::from(42_i64));
        map.insert("enabled".into(), Dynamic::from(true));

        let json = dynamic_to_json(&Dynamic::from(map.clone()));
        let result = json_to_dynamic(&json);

        let result_map = result.try_cast::<Map>().unwrap();
        assert_eq!(result_map.get("name").unwrap().clone().into_string().unwrap(), "test");
        assert_eq!(result_map.get("count").unwrap().clone().try_cast::<i64>().unwrap(), 42);
    }

    #[test]
    fn test_bindings_save_load() {
        let (_accessor, _store, _guard) = setup_test_context();
        let mut engine = Engine::new();
        register(&mut engine);

        // Test save and load
        let result: bool = engine.eval(r#"
            save_state("test:key1", #{ name: "pirate", defeats: 5 })
        "#).unwrap();
        assert!(result);

        let loaded: Map = engine.eval(r#"
            load_state("test:key1")
        "#).unwrap();
        assert_eq!(loaded.get("name").unwrap().clone().into_string().unwrap(), "pirate");
        assert_eq!(loaded.get("defeats").unwrap().clone().try_cast::<i64>().unwrap(), 5);
    }

    #[test]
    fn test_bindings_delete() {
        let (_accessor, _store, _guard) = setup_test_context();
        let mut engine = Engine::new();
        register(&mut engine);

        engine.eval::<bool>("save_state(\"test:key1\", 42)").unwrap();
        assert!(engine.eval::<bool>("state_exists(\"test:key1\")").unwrap());

        engine.eval::<bool>("delete_state(\"test:key1\")").unwrap();
        assert!(!engine.eval::<bool>("state_exists(\"test:key1\")").unwrap());
    }

    #[test]
    fn test_bindings_checkpoints() {
        let (_accessor, _store, _guard) = setup_test_context();
        let mut engine = Engine::new();
        register(&mut engine);

        engine.eval::<bool>("save_state(\"test:value\", \"original\")").unwrap();
        engine.eval::<bool>("checkpoint(\"cp1\")").unwrap();

        engine.eval::<bool>("save_state(\"test:value\", \"modified\")").unwrap();
        let value: String = engine.eval("load_state(\"test:value\")").unwrap();
        assert_eq!(value, "modified");

        engine.eval::<bool>("restore_checkpoint(\"cp1\")").unwrap();
        let value: String = engine.eval("load_state(\"test:value\")").unwrap();
        assert_eq!(value, "original");
    }
}
