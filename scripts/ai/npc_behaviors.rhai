// NPC AI Behaviors
// Defines decision-making for NPCs in combat and non-combat situations.
//
// Each behavior function returns an action map.

// ============================================================================
// COMBAT BEHAVIORS
// ============================================================================

/// Pirate behavior - aggressive but self-preserving
fn pirate_combat_ai(npc, targets, combat_state) {
    let hull = npc["hull"];
    let ammo = npc["ammunition"];
    let allies = combat_state["ally_count"];
    let enemies = combat_state["enemy_count"];

    // Flee if badly damaged or outnumbered and hurt
    if hull < 20.0 {
        return #{ action: "flee", reason: "critical damage" };
    }

    if hull < 40.0 && enemies > allies {
        return #{ action: "flee", reason: "outnumbered and damaged" };
    }

    // No ammo? Ram or flee
    if ammo < 5.0 {
        if hull > 50.0 && rand() < 0.3 {
            return #{ action: "ram", target: select_weakest(targets), reason: "out of ammo, ramming" };
        }
        return #{ action: "flee", reason: "out of ammunition" };
    }

    // Target selection: prefer weak targets
    let target = select_weakest(targets);

    // Aggressive attacks
    if hull > 70.0 {
        return #{ action: "attack", target: target, aggression: "high" };
    } else {
        return #{ action: "attack", target: target, aggression: "medium" };
    }
}

/// Sera behavior - relentless, no self-preservation
fn sera_combat_ai(npc, targets, combat_state) {
    let hull = npc["hull"];
    let ammo = npc["ammunition"];

    // Sera never flee - they fight to destruction
    if ammo < 5.0 {
        // Ram the nearest target
        return #{ action: "ram", target: select_nearest(targets, npc), reason: "ammunition depleted - ramming" };
    }

    // Always attack, maximum aggression
    let target = select_highest_threat(targets);
    return #{ action: "attack", target: target, aggression: "maximum" };
}

/// Civilian behavior - flee immediately
fn civilian_combat_ai(npc, targets, combat_state) {
    // Civilians always try to flee
    return #{ action: "flee", reason: "civilian - avoiding combat" };
}

/// Military behavior - tactical and coordinated
fn military_combat_ai(npc, targets, combat_state) {
    let hull = npc["hull"];
    let shields = npc["shields"];
    let ammo = npc["ammunition"];
    let allies = combat_state["ally_count"];
    let enemies = combat_state["enemy_count"];

    // Tactical retreat if badly outmatched
    if hull < 25.0 && allies < enemies {
        return #{ action: "retreat", reason: "tactical withdrawal" };
    }

    // Recharge shields if low and not critical
    if shields < 20.0 && hull > 50.0 {
        return #{ action: "defensive", reason: "recharging shields" };
    }

    // Focus fire on wounded enemies
    let target = select_most_damaged(targets);

    if target != () {
        return #{ action: "attack", target: target, aggression: "medium" };
    }

    // Default: attack highest threat
    let target = select_highest_threat(targets);
    return #{ action: "attack", target: target, aggression: "medium" };
}

/// Drone behavior - simple but coordinated
fn drone_combat_ai(npc, targets, combat_state) {
    let hull = npc["hull"];

    // Drones retreat when damaged
    if hull < 40.0 {
        return #{ action: "retreat", reason: "damage threshold reached" };
    }

    // Simple attack logic - nearest target
    let target = select_nearest(targets, npc);
    return #{ action: "attack", target: target, aggression: "medium" };
}

// ============================================================================
// TARGET SELECTION HELPERS
// ============================================================================

fn select_weakest(targets) {
    let weakest = ();
    let lowest_hull = 999.0;

    for target in targets {
        if target["hull"] < lowest_hull {
            lowest_hull = target["hull"];
            weakest = target;
        }
    }

    weakest
}

fn select_nearest(targets, npc) {
    let nearest = ();
    let min_dist = 999999.0;

    let my_x = npc["position_x"];
    let my_y = npc["position_y"];

    for target in targets {
        let dx = target["position_x"] - my_x;
        let dy = target["position_y"] - my_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < min_dist {
            min_dist = dist;
            nearest = target;
        }
    }

    nearest
}

fn select_highest_threat(targets) {
    let highest = ();
    let max_threat = 0.0;

    for target in targets {
        // Threat = attack power * remaining hull percentage
        let threat = target["attack"] * (target["hull"] / 100.0);
        if threat > max_threat {
            max_threat = threat;
            highest = target;
        }
    }

    highest
}

fn select_most_damaged(targets) {
    let most_damaged = ();
    let lowest_hull = 40.0; // Only target if below 40%

    for target in targets {
        if target["hull"] < lowest_hull {
            lowest_hull = target["hull"];
            most_damaged = target;
        }
    }

    most_damaged
}

// ============================================================================
// NON-COMBAT BEHAVIORS
// ============================================================================

/// Patrol behavior for sector NPCs
fn patrol_behavior(npc, sector) {
    let current_x = npc["position_x"];
    let current_y = npc["position_y"];
    let patrol_points = npc["patrol_points"];
    let current_point = npc["current_patrol_point"];

    if patrol_points.len() == 0 {
        return #{ action: "idle" };
    }

    let target_point = patrol_points[current_point];
    let dx = target_point["x"] - current_x;
    let dy = target_point["y"] - current_y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 10.0 {
        // Reached waypoint, move to next
        let next_point = (current_point + 1) % patrol_points.len();
        return #{
            action: "patrol",
            next_waypoint: next_point,
            wait_time: rand_int(5, 30)
        };
    }

    return #{
        action: "move",
        target_x: target_point["x"],
        target_y: target_point["y"]
    };
}

/// Trading behavior for merchant NPCs
fn trader_behavior(npc, sector) {
    let cargo_value = npc["cargo_value"];
    let fuel = npc["fuel"];
    let locations = sector["locations"];

    // Low fuel? Head to station
    if fuel < 20.0 {
        let station = find_nearest_station(npc, locations);
        if station != () {
            return #{
                action: "move_to_dock",
                target: station,
                reason: "low fuel"
            };
        }
    }

    // Full cargo? Head to sell
    if cargo_value > 80.0 {
        let station = find_nearest_station(npc, locations);
        if station != () {
            return #{
                action: "move_to_dock",
                target: station,
                reason: "selling cargo"
            };
        }
    }

    // Otherwise, patrol trade routes
    return patrol_behavior(npc, sector);
}

fn find_nearest_station(npc, locations) {
    let nearest = ();
    let min_dist = 999999.0;

    for loc in locations {
        if loc["type"] == "station" || loc["type"] == "civilian_station" {
            let dx = loc["position_x"] - npc["position_x"];
            let dy = loc["position_y"] - npc["position_y"];
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < min_dist {
                min_dist = dist;
                nearest = loc;
            }
        }
    }

    nearest
}

// ============================================================================
// FACTION-SPECIFIC REACTIONS
// ============================================================================

/// How an NPC reacts to detecting the player
fn react_to_player(npc, player, faction_standing) {
    let npc_faction = npc["faction"];

    // Hostile factions always attack
    if npc_faction == "sera" || npc_faction == "drone" {
        return #{ reaction: "attack", reason: "hostile faction" };
    }

    // Pirates attack if player is weak or valuable
    if npc_faction == "pirate" {
        let player_strength = player["hull"] * (player["ammunition"] / 100.0);
        let npc_strength = npc["hull"] * (npc["ammunition"] / 100.0);

        if player_strength < npc_strength * 0.7 {
            return #{ reaction: "attack", reason: "weak target" };
        }
        return #{ reaction: "avoid", reason: "target too strong" };
    }

    // Legitimate factions check standing
    if faction_standing < -50 {
        return #{ reaction: "attack", reason: "hostile reputation" };
    } else if faction_standing < -20 {
        return #{ reaction: "avoid", reason: "poor reputation" };
    } else if faction_standing > 50 {
        return #{ reaction: "friendly_hail", reason: "good reputation" };
    }

    return #{ reaction: "ignore", reason: "neutral" };
}

// ============================================================================
// COMBAT UTILITY FUNCTIONS
// ============================================================================

/// Calculate whether NPC should use special ability
fn should_use_ability(npc, ability_name, combat_state) {
    let cooldowns = npc["ability_cooldowns"];
    let cooldown = cooldowns[ability_name];

    if cooldown > 0 {
        return false;
    }

    // Different abilities have different triggers
    if ability_name == "emergency_shields" {
        return npc["shields"] < 10.0 && npc["hull"] > 30.0;
    }

    if ability_name == "overdrive" {
        let enemies = combat_state["enemy_count"];
        return enemies > 2 && npc["hull"] > 50.0;
    }

    if ability_name == "repair_nanites" {
        return npc["hull"] < 50.0;
    }

    false
}

/// Morale check for NPCs (can they keep fighting?)
fn check_morale(npc, combat_state) {
    let hull = npc["hull"];
    let allies_lost = combat_state["allies_lost"];
    let allies_remaining = combat_state["ally_count"];

    let morale = 100.0;

    // Hull damage hurts morale
    morale -= (100.0 - hull) * 0.5;

    // Losing allies hurts morale
    morale -= allies_lost * 15.0;

    // Being outnumbered hurts morale
    let enemy_count = combat_state["enemy_count"];
    if enemy_count > allies_remaining {
        morale -= (enemy_count - allies_remaining) * 10.0;
    }

    // Random factor
    morale += rand_float(-10.0, 10.0);

    morale = clamp(morale, 0.0, 100.0);

    if morale < 20.0 {
        let result = #{ broken: true, morale: morale };
        return result;
    }

    let result = #{ broken: false, morale: morale };
    result
}
