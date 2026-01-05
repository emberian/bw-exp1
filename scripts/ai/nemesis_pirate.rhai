// =============================================================================
// Nemesis Pirate AI
// =============================================================================
// A pirate that remembers past encounters with players and grows stronger.
// Demonstrates: Persistence system + Behavior Trees
//
// Features:
// - Remembers how many times defeated by each player
// - Gets tougher/smarter after each defeat
// - Taunts players it recognizes
// - Calls for backup after multiple defeats
// - Develops grudges and prioritizes nemesis targets

// =============================================================================
// PERSISTENCE KEYS
// =============================================================================

/// Get the persistence key for a player's nemesis history
fn nemesis_key(player_id) {
    `nemesis:${player_id}`
}

/// Load or create nemesis history for a player
fn load_nemesis_history(player_id) {
    let history = load_state(nemesis_key(player_id));
    if history == () {
        // First encounter - create new history
        #{
            name: generate_pirate_name(),
            defeats: 0,
            victories: 0,
            last_encounter: 0,
            grudge_level: 0,
            known_tactics: [],
            title: "Pirate",
        }
    } else {
        history
    }
}

/// Save nemesis history
fn save_nemesis_history(player_id, history) {
    save_state(nemesis_key(player_id), history);
}

/// Generate a random pirate name
fn generate_pirate_name() {
    let first_names = ["Blackbeard", "Red", "Iron", "Storm", "Dread", "Shadow", "Crimson"];
    let last_names = ["Jack", "Morgan", "Bones", "Cutlass", "Fury", "Reaver", "Corsair"];

    let first = first_names[rand_int(0, first_names.len() - 1)];
    let last = last_names[rand_int(0, last_names.len() - 1)];

    `${first} ${last}`
}

/// Get title based on defeats survived
fn get_title(defeats) {
    if defeats >= 5 { "Dread Pirate" }
    else if defeats >= 3 { "Notorious" }
    else if defeats >= 1 { "Veteran" }
    else { "Pirate" }
}

// =============================================================================
// BEHAVIOR TREE
// =============================================================================

fn build_behavior_tree() {
    bt_selector([
        // Priority 1: Critically damaged - flee but remember!
        bt_sequence([
            bt_condition("is_critically_damaged"),
            bt_action("flee_and_remember")
        ]),

        // Priority 2: Recognize nemesis and adjust tactics
        bt_sequence([
            bt_condition("has_nemesis_target"),
            bt_selector([
                // Call for backup if defeated too many times
                bt_sequence([
                    bt_condition("should_call_backup"),
                    bt_action("call_wingmen")
                ]),
                // Use learned counter-tactics
                bt_sequence([
                    bt_condition("knows_player_tactics"),
                    bt_action("counter_tactics")
                ]),
                // Standard nemesis engagement with extra aggression
                bt_action("engage_nemesis")
            ])
        ]),

        // Priority 3: Normal combat via utility AI
        bt_sequence([
            bt_condition("has_target"),
            bt_utility([
                #{
                    action: "aggressive_attack",
                    considerations: ["target_weakness", "own_strength", "grudge_bonus"],
                    weight: 1.0
                },
                #{
                    action: "flanking_attack",
                    considerations: ["target_distracted", "own_speed"],
                    weight: 0.8
                },
                #{
                    action: "cautious_attack",
                    considerations: ["own_weakness", "target_strength"],
                    weight: 0.6
                }
            ])
        ]),

        // Priority 4: Patrol and hunt
        bt_action("patrol_for_targets")
    ])
}

// =============================================================================
// CONDITIONS
// =============================================================================

fn is_critically_damaged(ctx) {
    let hull = ctx.hull ?? 100.0;
    hull < 15.0
}

fn has_nemesis_target(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return false; }

    // Check if we have history with this target
    let history = ctx.local_data.nemesis_history ?? #{};
    history.len() > 0 && history.defeats > 0
}

fn has_target(ctx) {
    let target = ctx.target ?? "";
    target != "" && target != ()
}

fn should_call_backup(ctx) {
    let history = ctx.local_data.nemesis_history ?? #{};
    let defeats = history.defeats ?? 0;

    // Call backup after 3+ defeats and we haven't already
    defeats >= 3 && !(ctx.local_data.backup_called ?? false)
}

fn knows_player_tactics(ctx) {
    let history = ctx.local_data.nemesis_history ?? #{};
    let known = history.known_tactics ?? [];
    known.len() > 0
}

fn target_distracted(ctx) {
    let target_focus = ctx.target_focus ?? "";
    let my_id = ctx.entity_id;
    target_focus != "" && target_focus != my_id
}

// =============================================================================
// UTILITY CONSIDERATIONS
// =============================================================================

fn target_weakness(ctx) {
    let target_hull = ctx.target_hull ?? 100.0;
    let target_max = ctx.target_max_hull ?? 100.0;
    curve_inverse_quadratic(consider_damage(target_hull, target_max))
}

fn own_strength(ctx) {
    let hull = ctx.hull ?? 100.0;
    let max_hull = ctx.max_hull ?? 100.0;
    consider_health(hull, max_hull)
}

fn own_weakness(ctx) {
    1.0 - own_strength(ctx)
}

fn own_speed(ctx) {
    let speed = ctx.speed ?? 50.0;
    clamp(speed / 100.0, 0.0, 1.0)
}

fn target_strength(ctx) {
    let target_hull = ctx.target_hull ?? 100.0;
    let target_max = ctx.target_max_hull ?? 100.0;
    consider_health(target_hull, target_max)
}

/// Bonus aggression against nemesis targets
fn grudge_bonus(ctx) {
    let history = ctx.local_data.nemesis_history ?? #{};
    let grudge = history.grudge_level ?? 0;

    // Each grudge level adds 10% bonus, max 50%
    clamp(grudge * 0.1, 0.0, 0.5) + 0.5
}

// =============================================================================
// ACTIONS
// =============================================================================

fn flee_and_remember(ctx) {
    let target_player = ctx.target_player_id ?? "";

    if target_player != "" {
        // Record this defeat
        let history = load_nemesis_history(target_player);
        history.defeats += 1;
        history.grudge_level += 1;
        history.last_encounter = ctx.game_time;
        history.title = get_title(history.defeats);

        // Learn from this defeat - remember what the player was using
        let player_stance = ctx.target_combat_stance ?? "balanced";
        if !history.known_tactics.contains(player_stance) {
            history.known_tactics.push(player_stance);
        }

        save_nemesis_history(target_player, history);

        log_info(`${history.title} ${history.name} flees, vowing revenge! (Defeat #${history.defeats})`);
    }

    #{ status: "success", action: "flee", reason: "survival" }
}

fn call_wingmen(ctx) {
    log_info("Nemesis calls for backup!");

    // Spawn 2 wingmen
    spawn_npc(#{
        class: "pirate_raider",
        position: #{ x: ctx.position.x + 50.0, y: ctx.position.y, z: ctx.position.z },
        faction: "pirate",
        behavior: "ai/pirate_bt.rhai"
    });

    spawn_npc(#{
        class: "pirate_raider",
        position: #{ x: ctx.position.x - 50.0, y: ctx.position.y, z: ctx.position.z },
        faction: "pirate",
        behavior: "ai/pirate_bt.rhai"
    });

    ctx.local_data.backup_called = true;

    #{ status: "success", action: "call_wingmen" }
}

fn counter_tactics(ctx) {
    let history = ctx.local_data.nemesis_history ?? #{};
    let known = history.known_tactics ?? [];

    // Adjust our stance to counter known player tactics
    let counter_stance = "balanced";
    if known.contains("aggressive") {
        counter_stance = "defensive";  // Let them overextend
    } else if known.contains("defensive") {
        counter_stance = "aggressive";  // Overwhelm them
    } else if known.contains("evasive") {
        counter_stance = "aggressive";  // Pin them down
    }

    set_combat_stance(ctx.entity_id, counter_stance);

    log_info(`Nemesis adapts tactics: switching to ${counter_stance}`);

    #{ status: "success", action: "counter_tactics", stance: counter_stance }
}

fn engage_nemesis(ctx) {
    let history = ctx.local_data.nemesis_history ?? #{};
    let target = ctx.target ?? "";

    // Extra aggressive against nemesis
    log_info(`${history.title} ${history.name} engages their nemesis with fury!`);

    #{ status: "success", action: "attack", target: target, aggression: "maximum" }
}

fn aggressive_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    #{ status: "success", action: "attack", target: target, aggression: "high" }
}

fn flanking_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    #{ status: "success", action: "flank", target: target }
}

fn cautious_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    #{ status: "success", action: "attack", target: target, aggression: "low" }
}

fn patrol_for_targets(ctx) {
    #{ status: "running", action: "patrol" }
}

// =============================================================================
// LIFECYCLE HOOKS
// =============================================================================

fn on_spawn(ctx) {
    log_info(`Nemesis pirate spawning for entity ${ctx.entity_id}`);

    // Build behavior tree
    let tree = build_behavior_tree();
    ctx.local_data.behavior_tree = tree;

    // Initialize state
    ctx.local_data.backup_called = false;
    ctx.local_data.nemesis_history = #{};

    ctx
}

fn on_update(ctx) {
    // Check if current target is a known nemesis
    let target_player = ctx.target_player_id ?? "";

    if target_player != "" {
        let history = load_nemesis_history(target_player);

        if history.defeats > 0 {
            ctx.local_data.nemesis_history = history;

            // Taunt on first detection this encounter
            if !(ctx.local_data.taunted ?? false) {
                let taunt = get_taunt(history);
                log_info(`${history.title} ${history.name}: "${taunt}"`);
                ctx.local_data.taunted = true;
            }
        }
    }

    ctx
}

fn on_destroy(ctx) {
    let target_player = ctx.target_player_id ?? "";

    if target_player != "" {
        // Record our victory if we won
        let our_hull = ctx.hull ?? 0.0;
        if our_hull > 0.0 {
            let history = load_nemesis_history(target_player);
            history.victories += 1;
            history.grudge_level = (history.grudge_level - 1).max(0);
            save_nemesis_history(target_player, history);

            log_info(`${history.title} ${history.name} claims victory!`);
        }
    }

    log_info(`Nemesis pirate destroyed for entity ${ctx.entity_id}`);
}

/// Get a taunt based on history
fn get_taunt(history) {
    let defeats = history.defeats;

    if defeats >= 5 {
        "We meet again! This time, I've brought friends..."
    } else if defeats >= 3 {
        "You again?! I've learned from our past... encounters."
    } else if defeats >= 1 {
        "I remember you! This time will be different!"
    } else {
        "Fresh meat!"
    }
}
