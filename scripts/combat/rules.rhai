// Combat Rules Script
//
// Customizable combat mechanics including target selection and flee calculations.
//
// These functions are OPTIONAL - if not defined, the server uses default Rust logic.

// Select a target from available enemies.
//
// Context (ctx) contains:
//   - attacker: Map with ship data (id, name, hull, shields, speed, etc.)
//   - targets: Array of enemy ship data maps
//   - combat_round: i64
//
// Returns: String (target UUID) or () for no target
//
// Default behavior: random selection
// Advanced options: prioritize low HP, highest threat, nearest, etc.
fn select_target(ctx) {
    let targets = ctx["targets"];

    if targets.is_empty() {
        return ();
    }

    // Default: random target selection (matches current Rust behavior)
    let idx = floor(rand() * targets.len());
    if idx >= targets.len() {
        idx = targets.len() - 1;
    }

    targets[idx]["id"]
}

// Calculate flee success chance for a ship.
//
// Context (ctx) contains:
//   - ship: Map with ship data (speed, hull, shields, etc.)
//   - engagement: Map with combat data
//   - max_flee_chance: f64 (from config)
//   - flee_speed_divisor: f64 (from config)
//
// Returns: f64 (0.0 to 1.0 flee probability)
fn calculate_flee_chance(ctx) {
    let ship = ctx["ship"];
    let max_chance = ctx["max_flee_chance"];
    let divisor = ctx["flee_speed_divisor"];

    let speed = ship["speed"];
    if speed == () {
        speed = 30.0;  // Default speed fallback
    }

    // Base calculation: speed / divisor, capped at max_chance
    let chance = speed / divisor;

    if chance > max_chance {
        chance = max_chance;
    }

    if chance < 0.0 {
        chance = 0.0;
    }

    chance
}

// Calculate attack damage, hit chance, and critical hits.
//
// Context (ctx) contains:
//   - attacker: Map with combat stats (attack, defense, speed)
//   - defender: Map with combat stats (attack, defense, speed)
//   - weapon_damage: f64
//   - weapon_accuracy: f64
//
// Returns: Map { hit: bool, damage: f64, critical_hit: bool, ammo_consumed: f64 }
fn calculate_attack(ctx) {
    let attacker = ctx["attacker"];
    let defender = ctx["defender"];
    let weapon_damage = ctx["weapon_damage"];
    let weapon_accuracy = ctx["weapon_accuracy"];

    // Calculate hit chance: base accuracy adjusted by attacker/defender stats
    let attack_bonus = attacker["attack"] / 100.0;
    let defense_penalty = defender["defense"] / 200.0;
    let speed_factor = (attacker["speed"] - defender["speed"]) / 200.0;

    let hit_chance = weapon_accuracy + attack_bonus - defense_penalty + speed_factor;

    // Clamp hit chance between 0.1 and 0.95
    if hit_chance < 0.1 {
        hit_chance = 0.1;
    }
    if hit_chance > 0.95 {
        hit_chance = 0.95;
    }

    // Roll for hit
    let hit_roll = rand();
    let hit = hit_roll < hit_chance;

    if !hit {
        return #{
            hit: false,
            damage: 0.0,
            critical_hit: false,
            ammo_consumed: 1.0
        };
    }

    // Calculate damage
    let base_damage = weapon_damage;
    let attack_mult = 1.0 + (attacker["attack"] / 100.0);
    let defense_reduction = defender["defense"] / 200.0;

    let damage = base_damage * attack_mult * (1.0 - defense_reduction);

    // Variance: +/- 20%
    let variance = 0.8 + (rand() * 0.4);
    damage = damage * variance;

    // Critical hit check (10% base chance, +1% per 10 attack over 50)
    let crit_chance = 0.10;
    if attacker["attack"] > 50.0 {
        crit_chance = crit_chance + ((attacker["attack"] - 50.0) / 1000.0);
    }
    if crit_chance > 0.30 {
        crit_chance = 0.30;  // Cap at 30%
    }

    let critical_hit = rand() < crit_chance;
    if critical_hit {
        damage = damage * 2.0;
    }

    // Ensure minimum damage of 1
    if damage < 1.0 {
        damage = 1.0;
    }

    #{
        hit: true,
        damage: damage,
        critical_hit: critical_hit,
        ammo_consumed: 1.0
    }
}

// Example: Advanced target selection (prioritize lowest HP)
// Uncomment to use instead of random selection:
//
// fn select_target_lowest_hp(ctx) {
//     let targets = ctx["targets"];
//     if targets.is_empty() {
//         return ();
//     }
//
//     let lowest_hp = 999999.0;
//     let best_target = ();
//
//     for t in targets {
//         let hp = t["hull"];
//         if hp < lowest_hp {
//             lowest_hp = hp;
//             best_target = t["id"];
//         }
//     }
//
//     best_target
// }

// Example: Focus fire on already-damaged enemies
// fn select_target_finish_off(ctx) {
//     let targets = ctx["targets"];
//     if targets.is_empty() {
//         return ();
//     }
//
//     // First look for damaged targets (< 50% hull)
//     for t in targets {
//         if t["hull"] < 50.0 {
//             return t["id"];
//         }
//     }
//
//     // Fall back to random
//     let idx = floor(rand() * targets.len());
//     targets[idx]["id"]
// }
