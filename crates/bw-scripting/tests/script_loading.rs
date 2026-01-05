//! Integration tests for script loading

use bw_scripting::ScriptEngine;

#[test]
fn test_engine_creation() {
    let engine = ScriptEngine::new("../../scripts");
    assert!(engine.loaded_scripts().is_empty());
}

#[test]
fn test_basic_eval() {
    let engine = ScriptEngine::new("../../scripts");
    let result: i64 = engine.eval("2 + 2").unwrap();
    assert_eq!(result, 4);
}

#[test]
fn test_utility_functions() {
    let engine = ScriptEngine::new("../../scripts");

    // Test clamp function
    let result: f64 = engine.eval("clamp(150.0, 0.0, 100.0)").unwrap();
    assert!((result - 100.0).abs() < 0.01);

    // Test lerp function
    let result: f64 = engine.eval("lerp(0.0, 100.0, 0.5)").unwrap();
    assert!((result - 50.0).abs() < 0.01);
}

#[test]
fn test_random_functions() {
    let engine = ScriptEngine::new("../../scripts");

    // Test rand() returns value between 0 and 1
    let result: f64 = engine.eval("rand()").unwrap();
    assert!(result >= 0.0 && result < 1.0);

    // Test rand_int returns value in range
    let result: i64 = engine.eval("rand_int(1, 10)").unwrap();
    assert!(result >= 1 && result <= 10);
}

#[test]
fn test_uuid_generation() {
    let engine = ScriptEngine::new("../../scripts");

    let result: String = engine.eval("uuid()").unwrap();
    assert_eq!(result.len(), 36); // UUID format: 8-4-4-4-12
}

#[test]
#[ignore] // Run manually when scripts exist
fn test_load_mission_scripts() {
    let engine = ScriptEngine::new("../../scripts");

    // This will fail gracefully if scripts don't exist
    if let Err(e) = engine.load_all_scripts() {
        println!("Script loading error (expected if running from wrong dir): {}", e);
    } else {
        let scripts = engine.loaded_scripts();
        println!("Loaded {} scripts", scripts.len());
        for script in &scripts {
            println!("  - {}", script);
        }
        assert!(!scripts.is_empty());
    }
}
