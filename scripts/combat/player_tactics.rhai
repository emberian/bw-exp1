// Player Tactics Script
//
// Combat helpers for player ships. Provides functions for:
// - Smart target selection based on situation
// - Recommended actions based on ship state
// - Combat analysis and threat assessment

// Analyze the combat situation and return recommendations for the player.
//
// Context (ctx) contains:
//   - player_ship: Map with player's ship data
//   - enemies: Array of enemy ship data
//   - allies: Array of friendly ship data
//   - engagement: Map with combat engagement data
//
// Returns: Map with recommendations
fn analyze_combat_situation(ctx) {
    let player = ctx["player_ship"];
    let enemies = ctx["enemies"];
    let allies = ctx["allies"];

    let player_hull = player["hull"];
    let stance = player["combat_stance"];

    // Count threats by severity
    let high_threat = 0;
    let total_enemy_hp = 0.0;
    let weakest_enemy = ();
    let weakest_hp = 999999.0;
    let strongest_enemy = ();
    let strongest_hp = 0.0;

    for enemy in enemies {
        let hp = enemy["hull"];
        total_enemy_hp += hp;

        if hp < weakest_hp {
            weakest_hp = hp;
            weakest_enemy = enemy;
        }
        if hp > strongest_hp {
            strongest_hp = hp;
            strongest_enemy = enemy;
        }

        // Count high threat enemies (> 70 HP)
        if hp > 70.0 {
            high_threat += 1;
        }
    }

    // Calculate threat level
    let threat_level = "low";
    if enemies.len() > 3 || high_threat > 1 {
        threat_level = "high";
    } else if enemies.len() > 1 || high_threat > 0 {
        threat_level = "medium";
    }

    // Determine recommended action
    let recommended_action = "attack";
    let recommended_target = ();
    let recommended_stance = stance;

    // Critical health - consider fleeing
    if player_hull < 25.0 {
        if stance != "evasive" {
            recommended_stance = "evasive";
        }
        recommended_action = "flee";
    }
    // Low health - defensive
    else if player_hull < 50.0 {
        if stance != "defensive" && stance != "evasive" {
            recommended_stance = "defensive";
        }
        // Focus on finishing weak enemies to reduce incoming damage
        if weakest_enemy != () && weakest_hp < 30.0 {
            recommended_target = weakest_enemy["id"];
            recommended_action = "attack";
        }
    }
    // Healthy - can be aggressive
    else if player_hull > 75.0 {
        if threat_level == "low" && stance != "aggressive" {
            recommended_stance = "aggressive";
        }
        // Focus fire on weakest to eliminate threats
        if weakest_enemy != () {
            recommended_target = weakest_enemy["id"];
        }
    }
    // Default: balanced approach
    else {
        recommended_stance = "balanced";
        if weakest_enemy != () {
            recommended_target = weakest_enemy["id"];
        }
    }

    // If no target selected, use strongest (highest threat)
    if recommended_target == () && strongest_enemy != () {
        recommended_target = strongest_enemy["id"];
    }

    #{
        threat_level: threat_level,
        enemy_count: enemies.len(),
        ally_count: allies.len(),
        total_enemy_hp: total_enemy_hp,
        player_hull_percent: player_hull,
        recommended_action: recommended_action,
        recommended_target: recommended_target,
        recommended_stance: recommended_stance,
        current_stance: stance,
        weakest_enemy_id: if weakest_enemy != () { weakest_enemy["id"] } else { () },
        weakest_enemy_hp: weakest_hp,
        should_flee: player_hull < 25.0 && threat_level != "low"
    }
}

// Select the optimal target based on player's current stance and situation.
//
// Arguments:
//   - player_id: String (player's ship ID)
//   - enemies: Array of enemy ship data
//
// Returns: String (target UUID) or () for no target
fn select_optimal_target(player_id, enemies) {
    if enemies.is_empty() {
        return ();
    }

    // Check if player has a locked target
    let locked = get_locked_target(player_id);
    if locked != "" && locked != () {
        // Verify it's still a valid target
        for enemy in enemies {
            if enemy["id"] == locked {
                return locked;
            }
        }
    }

    // Get player's stance
    let stance = get_combat_stance(player_id);

    // Stance-based target selection
    if stance == "aggressive" {
        // Finish off weakest enemy
        return find_weakest_target(enemies);
    } else if stance == "defensive" {
        // Focus on highest threat
        return find_highest_threat_target(enemies);
    } else if stance == "evasive" {
        // Don't engage, return () to skip attack
        return ();
    }

    // Balanced: prioritize medium-health targets
    return find_best_value_target(enemies);
}

// Find the enemy with lowest HP
fn find_weakest_target(enemies) {
    let weakest_hp = 999999.0;
    let target_id = ();

    for enemy in enemies {
        let hp = enemy["hull"];
        if hp < weakest_hp {
            weakest_hp = hp;
            target_id = enemy["id"];
        }
    }

    target_id
}

// Find the enemy with highest threat (approximated by stats)
fn find_highest_threat_target(enemies) {
    let highest_threat = 0.0;
    let target_id = ();

    for enemy in enemies {
        // Threat = attack + (hull / 2)
        let attack = enemy["attack"];
        if attack == () { attack = 50.0; }
        let threat = attack + (enemy["hull"] / 2.0);

        if threat > highest_threat {
            highest_threat = threat;
            target_id = enemy["id"];
        }
    }

    target_id
}

// Find the best "value" target (medium HP, can be killed quickly)
fn find_best_value_target(enemies) {
    let best_score = 0.0;
    let target_id = ();

    for enemy in enemies {
        let hp = enemy["hull"];
        // Score: lower HP is better, but not too low (waste of turn)
        // Sweet spot around 30-60 HP
        let score = 100.0 - abs(hp - 45.0);

        if score > best_score {
            best_score = score;
            target_id = enemy["id"];
        }
    }

    if target_id == () && enemies.len() > 0 {
        target_id = enemies[0]["id"];
    }

    target_id
}

// Helper: absolute value
fn abs(x) {
    if x < 0.0 { -x } else { x }
}

// Calculate recommended stance based on situation
//
// Arguments:
//   - player_hull: f64 (current hull %)
//   - enemy_count: i64
//   - ally_count: i64
//
// Returns: String (stance name)
fn recommend_stance(player_hull, enemy_count, ally_count) {
    // Critical - flee mode
    if player_hull < 20.0 {
        return "evasive";
    }

    // Low health and outnumbered
    if player_hull < 40.0 && enemy_count > ally_count {
        return "defensive";
    }

    // Healthy and advantage
    if player_hull > 70.0 && ally_count >= enemy_count {
        return "aggressive";
    }

    // Default balanced
    "balanced"
}

// Evaluate whether fleeing is advisable
//
// Arguments:
//   - player_ship: Map with player ship data
//   - enemies: Array of enemy ship data
//
// Returns: Map { should_flee: bool, flee_chance: f64, reason: String }
fn evaluate_flee(player_ship, enemies) {
    let hull = player_ship["hull"];
    let shields = player_ship["shields"];
    let speed = player_ship["speed"];
    if speed == () { speed = 50.0; }

    // Calculate total enemy threat
    let threat_sum = 0.0;
    for enemy in enemies {
        let enemy_attack = enemy["attack"];
        if enemy_attack == () { enemy_attack = 50.0; }
        threat_sum += enemy_attack;
    }

    // Base flee chance from speed
    let flee_chance = speed / 100.0;
    if flee_chance > 0.7 { flee_chance = 0.7; }

    let should_flee = false;
    let reason = "Situation manageable";

    // Critical hull - should flee
    if hull < 15.0 {
        should_flee = true;
        reason = "Critical hull damage";
    }
    // Low hull and high threat
    else if hull < 30.0 && threat_sum > 100.0 {
        should_flee = true;
        reason = "Low hull and overwhelming enemy firepower";
    }
    // Very outnumbered
    else if enemies.len() > 4 && hull < 60.0 {
        should_flee = true;
        reason = "Heavily outnumbered";
    }

    #{
        should_flee: should_flee,
        flee_chance: flee_chance,
        reason: reason,
        enemy_threat: threat_sum,
        player_hull: hull
    }
}

// Send combat status notification to player
fn notify_combat_status(player_id, status_type, message) {
    if status_type == "warning" {
        send_warning(player_id, message);
    } else if status_type == "danger" {
        send_error(player_id, message);
    } else {
        send_notification(player_id, message);
    }
}

// Send combat recommendations to player
fn send_combat_recommendations(player_id, analysis) {
    if analysis["should_flee"] {
        send_warning(player_id, "Critical situation! Consider fleeing. Hull at " + analysis["player_hull_percent"] + "%");
    } else if analysis["threat_level"] == "high" {
        send_notification_type(player_id, "High threat combat. " + analysis["enemy_count"] + " enemies detected.", "warning");
    }

    if analysis["recommended_stance"] != analysis["current_stance"] {
        send_notification(player_id, "Recommended stance: " + analysis["recommended_stance"]);
    }
}
