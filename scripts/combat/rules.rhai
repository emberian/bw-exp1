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
