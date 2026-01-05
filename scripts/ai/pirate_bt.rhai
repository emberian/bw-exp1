// =============================================================================
// Pirate AI Behavior Tree
// =============================================================================
// Demonstrates the behavior tree + utility AI system for NPC decision making.
//
// This script defines:
// 1. The behavior tree structure
// 2. Condition functions (return true/false)
// 3. Action functions (return success/failure/running)
// 4. Utility consideration functions (return 0.0-1.0)

// =============================================================================
// BEHAVIOR TREE DEFINITION
// =============================================================================

/// Build the pirate behavior tree
/// Called on_spawn to create the tree
fn build_behavior_tree() {
    bt_selector([
        // Priority 1: Critical survival
        bt_sequence([
            bt_condition("is_critically_damaged"),
            bt_action("flee")
        ]),

        // Priority 2: Tactical retreat if outmatched
        bt_sequence([
            bt_condition("should_retreat"),
            bt_action("retreat")
        ]),

        // Priority 3: Out of ammo - ram or flee
        bt_sequence([
            bt_condition("is_out_of_ammo"),
            bt_selector([
                bt_sequence([
                    bt_condition("can_ram"),
                    bt_action("ram")
                ]),
                bt_action("flee")
            ])
        ]),

        // Priority 4: Combat decisions via utility AI
        bt_sequence([
            bt_condition("has_target"),
            bt_utility([
                #{
                    action: "attack_aggressive",
                    considerations: ["target_weakness", "own_strength", "ammo_available"],
                    weight: 1.0
                },
                #{
                    action: "attack_cautious",
                    considerations: ["target_weakness", "own_weakness"],
                    weight: 0.8
                },
                #{
                    action: "flank_target",
                    considerations: ["target_distracted", "own_strength"],
                    weight: 0.7
                }
            ])
        ]),

        // Priority 5: No combat - patrol
        bt_action("patrol")
    ])
}

// =============================================================================
// CONDITIONS
// =============================================================================
// All conditions receive ctx (context map) and return true/false

/// Check if hull is critically low
fn is_critically_damaged(ctx) {
    let hull = ctx.hull ?? 100.0;
    hull < 20.0
}

/// Check if we should tactically retreat
fn should_retreat(ctx) {
    let hull = ctx.hull ?? 100.0;
    let allies = ctx.allies ?? 0;
    let enemies = ctx.enemies ?? 0;

    // Retreat if damaged and outnumbered
    hull < 40.0 && enemies > allies
}

/// Check if out of ammunition
fn is_out_of_ammo(ctx) {
    let ammo = ctx.ammo ?? 100.0;
    ammo < 5.0
}

/// Check if we can attempt a ram attack
fn can_ram(ctx) {
    let hull = ctx.hull ?? 0.0;
    // Only ram if we have decent hull and feel brave
    hull > 50.0 && rand() < 0.3
}

/// Check if we have a valid target
fn has_target(ctx) {
    let target = ctx.target;
    target != () && target != ""
}

/// Check if target is distracted (fighting someone else)
fn target_distracted(ctx) {
    let target_focus = ctx.target_focus ?? "";
    let my_id = ctx.entity_id;
    target_focus != "" && target_focus != my_id
}

// =============================================================================
// ACTIONS
// =============================================================================
// All actions receive ctx and return "success", "failure", or "running"
// They can also return true/false or #{status: "..."} with additional data

/// Flee from combat
fn flee(ctx) {
    log_info(`Pirate ${ctx.entity_id} fleeing!`);
    // The actual flee logic would be handled by the game engine
    // based on this action being selected
    #{ status: "success", action: "flee", reason: "survival" }
}

/// Tactical retreat
fn retreat(ctx) {
    log_info(`Pirate ${ctx.entity_id} retreating tactically`);
    #{ status: "success", action: "retreat", reason: "tactical" }
}

/// Ram the target
fn ram(ctx) {
    let target = ctx.target ?? "";
    if target == "" {
        return "failure";
    }
    log_info(`Pirate ${ctx.entity_id} ramming ${target}!`);
    #{ status: "success", action: "ram", target: target }
}

/// Aggressive attack
fn attack_aggressive(ctx) {
    let target = ctx.target ?? "";
    if target == "" {
        return "failure";
    }
    #{ status: "success", action: "attack", target: target, aggression: "high" }
}

/// Cautious attack
fn attack_cautious(ctx) {
    let target = ctx.target ?? "";
    if target == "" {
        return "failure";
    }
    #{ status: "success", action: "attack", target: target, aggression: "medium" }
}

/// Flanking maneuver
fn flank_target(ctx) {
    let target = ctx.target ?? "";
    if target == "" {
        return "failure";
    }
    #{ status: "success", action: "flank", target: target }
}

/// Patrol behavior when not in combat
fn patrol(ctx) {
    // Return "running" to indicate ongoing patrol
    #{ status: "running", action: "patrol" }
}

// =============================================================================
// UTILITY CONSIDERATIONS
// =============================================================================
// All considerations receive ctx and return a score from 0.0 to 1.0
// Higher scores = more desirable

/// How weak is our target? (1.0 = very weak, 0.0 = full health)
fn target_weakness(ctx) {
    let target_hull = ctx.target_hull ?? 100.0;
    let target_max_hull = ctx.target_max_hull ?? 100.0;
    curve_inverse_quadratic(consider_damage(target_hull, target_max_hull))
}

/// How strong are we? (1.0 = full health, 0.0 = critical)
fn own_strength(ctx) {
    let hull = ctx.hull ?? 100.0;
    let max_hull = ctx.max_hull ?? 100.0;
    consider_health(hull, max_hull)
}

/// How weak are we? (1.0 = critical, 0.0 = full health)
fn own_weakness(ctx) {
    1.0 - own_strength(ctx)
}

/// How much ammo do we have? (1.0 = full, 0.0 = empty)
fn ammo_available(ctx) {
    let ammo = ctx.ammo ?? 100.0;
    let max_ammo = ctx.max_ammo ?? 100.0;
    consider_resource(ammo, max_ammo)
}

/// Is our target fighting someone else?
fn target_distracted_score(ctx) {
    if target_distracted(ctx) { 0.8 } else { 0.2 }
}

// =============================================================================
// LIFECYCLE HOOKS
// =============================================================================

/// Called when behavior is first attached
fn on_spawn(ctx) {
    log_info(`Pirate AI spawned for entity ${ctx.entity_id}`);

    // Store the behavior tree in local data
    let tree = build_behavior_tree();
    ctx.local_data.behavior_tree = tree;

    // Initialize state tracking
    ctx.local_data.patrol_waypoint = 0;
    ctx.local_data.last_target = "";

    // Return updated context
    ctx
}

/// Called every tick
fn on_update(ctx) {
    // The behavior tree is automatically executed by BehaviorManager
    // This hook can be used for additional per-tick logic

    // Track combat state for considerations
    ctx.local_data.in_combat = ctx.target != () && ctx.target != "";

    ctx
}

/// Called when behavior is destroyed
fn on_destroy(ctx) {
    log_info(`Pirate AI destroyed for entity ${ctx.entity_id}`);
}
