//! Combat API bindings for Rhai

use rhai::{Engine, Map};

/// Register combat API functions.
pub fn register(engine: &mut Engine) {
    // Calculate damage
    engine.register_fn("calculate_damage", |base_damage: f64, attack_stat: f64, defense_stat: f64| -> f64 {
        let attack_mult = attack_stat / 100.0;
        let defense_reduction = (defense_stat / 200.0).min(0.5);
        base_damage * attack_mult * (1.0 - defense_reduction)
    });

    // Calculate hit chance
    engine.register_fn("calculate_hit_chance", |base_accuracy: f64, attacker_speed: f64, defender_speed: f64| -> f64 {
        let speed_mod = attacker_speed / defender_speed.max(1.0);
        (base_accuracy * speed_mod).clamp(0.1, 0.95)
    });

    // Roll for hit
    engine.register_fn("roll_hit", |hit_chance: f64| -> bool {
        use rand::Rng;
        rand::thread_rng().r#gen::<f64>() < hit_chance
    });

    // Roll for critical
    engine.register_fn("roll_critical", |base_crit_chance: f64, experience: i64| -> bool {
        use rand::Rng;
        let exp_bonus = (experience as f64 / 1000.0).min(0.15);
        let crit_chance = base_crit_chance + exp_bonus;
        rand::thread_rng().r#gen::<f64>() < crit_chance
    });

    // Apply critical multiplier
    engine.register_fn("apply_critical", |damage: f64, is_critical: bool| -> f64 {
        if is_critical { damage * 1.5 } else { damage }
    });

    // Create combat result
    engine.register_fn("create_attack_result", |hit: bool, damage: f64, critical: bool| -> Map {
        let mut result = Map::new();
        result.insert("hit".into(), hit.into());
        result.insert("damage".into(), damage.into());
        result.insert("critical".into(), critical.into());
        result
    });

    // Full attack calculation
    engine.register_fn("resolve_attack", |
        base_damage: f64,
        base_accuracy: f64,
        attacker_attack: f64,
        attacker_speed: f64,
        attacker_experience: i64,
        defender_defense: f64,
        defender_speed: f64,
    | -> Map {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Hit check
        let hit_chance = (base_accuracy * (attacker_speed / defender_speed.max(1.0))).clamp(0.1, 0.95);
        let hit = rng.r#gen::<f64>() < hit_chance;

        if !hit {
            let mut result = Map::new();
            result.insert("hit".into(), false.into());
            result.insert("damage".into(), 0.0f64.into());
            result.insert("critical".into(), false.into());
            return result;
        }

        // Damage calculation
        let attack_mult = attacker_attack / 100.0;
        let defense_reduction = (defender_defense / 200.0).min(0.5);
        let base = base_damage * attack_mult * (1.0 - defense_reduction);

        // Critical check
        let crit_chance = 0.05 + (attacker_experience as f64 / 1000.0).min(0.15);
        let critical = rng.r#gen::<f64>() < crit_chance;
        let final_damage = if critical { base * 1.5 } else { base };

        let mut result = Map::new();
        result.insert("hit".into(), true.into());
        result.insert("damage".into(), final_damage.into());
        result.insert("critical".into(), critical.into());
        result
    });
}
