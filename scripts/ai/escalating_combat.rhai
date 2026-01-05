// =============================================================================
// Escalating Combat AI
// =============================================================================
// An enemy that escalates tactics as combat progresses.
// Demonstrates: Property Watches + Behavior Trees
//
// Features:
// - Watches own health and reacts to damage thresholds
// - Escalates aggression as time passes
// - Calls reinforcements when critically damaged
// - Enters "berserk" mode at low health
// - Monitors target and adapts tactics

// =============================================================================
// COMBAT PHASES
// =============================================================================

// Combat escalates through phases:
// 1. PROBING    - Cautious, testing the waters
// 2. ENGAGED    - Normal combat
// 3. AGGRESSIVE - Taking damage, fighting harder
// 4. DESPERATE  - Low health, calling for help
// 5. BERSERK    - Critical health, all-out attack

fn get_phase_name(phase) {
    switch phase {
        1 => "PROBING",
        2 => "ENGAGED",
        3 => "AGGRESSIVE",
        4 => "DESPERATE",
        5 => "BERSERK",
        _ => "UNKNOWN"
    }
}

// =============================================================================
// BEHAVIOR TREE
// =============================================================================

fn build_behavior_tree() {
    bt_selector([
        // Priority 1: Berserk mode - all out attack
        bt_sequence([
            bt_condition("is_berserk"),
            bt_action("berserk_attack")
        ]),

        // Priority 2: Desperate - call for help
        bt_sequence([
            bt_condition("is_desperate"),
            bt_selector([
                bt_sequence([
                    bt_condition("can_call_reinforcements"),
                    bt_action("call_reinforcements")
                ]),
                bt_action("desperate_attack")
            ])
        ]),

        // Priority 3: Aggressive phase
        bt_sequence([
            bt_condition("is_aggressive"),
            bt_utility([
                #{
                    action: "focused_fire",
                    considerations: ["target_weakness", "weapon_ready"],
                    weight: 1.0
                },
                #{
                    action: "power_attack",
                    considerations: ["energy_available", "opening_detected"],
                    weight: 0.9
                }
            ])
        ]),

        // Priority 4: Engaged phase
        bt_sequence([
            bt_condition("is_engaged"),
            bt_utility([
                #{
                    action: "standard_attack",
                    considerations: ["target_in_range", "weapon_ready"],
                    weight: 1.0
                },
                #{
                    action: "flanking_maneuver",
                    considerations: ["target_distracted", "mobility"],
                    weight: 0.7
                }
            ])
        ]),

        // Priority 5: Probing phase
        bt_sequence([
            bt_condition("is_probing"),
            bt_action("probe_defenses")
        ]),

        // Default: Engage
        bt_action("engage_target")
    ])
}

// =============================================================================
// WATCH SETUP
// =============================================================================

/// Set up property watches for combat escalation
fn setup_watches(ctx) {
    let entity_id = ctx.entity_id;

    // Watch for health dropping below 75% - escalate to AGGRESSIVE
    watch_property(
        entity_id,
        "hull",
        "less_than",
        75.0,
        "on_health_75"
    );

    // Watch for health dropping below 40% - escalate to DESPERATE
    watch_property(
        entity_id,
        "hull",
        "less_than",
        40.0,
        "on_health_40"
    );

    // Watch for health dropping below 15% - escalate to BERSERK
    watch_property(
        entity_id,
        "hull",
        "less_than",
        15.0,
        "on_health_15"
    );

    // Watch for shields going down completely
    watch_once(
        entity_id,
        "shields",
        "less_than",
        1.0,
        "on_shields_down"
    );

    log_info(`Combat watches set up for entity ${entity_id}`);
}

// =============================================================================
// WATCH CALLBACKS
// =============================================================================

/// Called when health drops below 75%
fn on_health_75(ctx) {
    if ctx.local_data.combat_phase < 3 {
        ctx.local_data.combat_phase = 3;
        log_info(`${ctx.entity_id} escalating to AGGRESSIVE phase!`);

        emit_event("combat_escalation", #{
            entity_id: ctx.entity_id,
            phase: "aggressive",
            hull: ctx.hull
        });
    }
    ctx
}

/// Called when health drops below 40%
fn on_health_40(ctx) {
    if ctx.local_data.combat_phase < 4 {
        ctx.local_data.combat_phase = 4;
        log_info(`${ctx.entity_id} escalating to DESPERATE phase!`);

        emit_event("combat_escalation", #{
            entity_id: ctx.entity_id,
            phase: "desperate",
            hull: ctx.hull
        });

        // Try to call for reinforcements
        ctx.local_data.reinforcements_requested = true;
    }
    ctx
}

/// Called when health drops below 15%
fn on_health_15(ctx) {
    if ctx.local_data.combat_phase < 5 {
        ctx.local_data.combat_phase = 5;
        log_info(`${ctx.entity_id} entering BERSERK mode!`);

        emit_event("combat_escalation", #{
            entity_id: ctx.entity_id,
            phase: "berserk",
            hull: ctx.hull
        });
    }
    ctx
}

/// Called when shields go down (one-shot)
fn on_shields_down(ctx) {
    log_info(`${ctx.entity_id}: Shields DOWN! Escalating tactics!`);

    // Immediately bump up one phase if not already aggressive
    if ctx.local_data.combat_phase < 3 {
        ctx.local_data.combat_phase = 3;
    }

    emit_event("shields_lost", #{
        entity_id: ctx.entity_id
    });

    ctx
}

// =============================================================================
// CONDITIONS
// =============================================================================

fn is_berserk(ctx) {
    ctx.local_data.combat_phase >= 5
}

fn is_desperate(ctx) {
    ctx.local_data.combat_phase == 4
}

fn is_aggressive(ctx) {
    ctx.local_data.combat_phase == 3
}

fn is_engaged(ctx) {
    ctx.local_data.combat_phase == 2
}

fn is_probing(ctx) {
    ctx.local_data.combat_phase == 1
}

fn can_call_reinforcements(ctx) {
    let requested = ctx.local_data.reinforcements_requested ?? false;
    let arrived = ctx.local_data.reinforcements_arrived ?? false;

    requested && !arrived
}

// =============================================================================
// UTILITY CONSIDERATIONS
// =============================================================================

fn target_weakness(ctx) {
    let target_hull = ctx.target_hull ?? 100.0;
    let target_max = ctx.target_max_hull ?? 100.0;
    curve_inverse_quadratic(consider_damage(target_hull, target_max))
}

fn weapon_ready(ctx) {
    let cooldown = ctx.weapon_cooldown ?? 0.0;
    if cooldown <= 0.0 { 1.0 } else { 0.0 }
}

fn energy_available(ctx) {
    let energy = ctx.energy ?? 100.0;
    let max_energy = ctx.max_energy ?? 100.0;
    consider_resource(energy, max_energy)
}

fn opening_detected(ctx) {
    let target_firing = ctx.target_is_firing ?? false;
    let target_shields_low = (ctx.target_shields ?? 100.0) < 30.0;

    if target_shields_low { 0.9 }
    else if target_firing { 0.6 }  // They're committed
    else { 0.3 }
}

fn target_in_range(ctx) {
    let distance = ctx.target_distance ?? 1000.0;
    let weapon_range = ctx.weapon_range ?? 500.0;

    if distance <= weapon_range { 1.0 }
    else if distance <= weapon_range * 1.5 { 0.5 }
    else { 0.0 }
}

fn target_distracted(ctx) {
    let target_focus = ctx.target_focus ?? "";
    let my_id = ctx.entity_id;

    if target_focus != "" && target_focus != my_id { 0.8 }
    else { 0.2 }
}

fn mobility(ctx) {
    let speed = ctx.speed ?? 50.0;
    clamp(speed / 100.0, 0.0, 1.0)
}

// =============================================================================
// ACTIONS
// =============================================================================

fn berserk_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id} BERSERK: All-out attack on ${target}!`);

    // Maximum aggression, ignore defense
    set_combat_stance(ctx.entity_id, "aggressive");

    emit_event("berserk_attack", #{
        attacker: ctx.entity_id,
        target: target
    });

    #{ status: "success", action: "berserk_attack", target: target, damage_multiplier: 1.5 }
}

fn desperate_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id} DESPERATE: Fighting for survival!`);

    #{ status: "success", action: "desperate_attack", target: target, damage_multiplier: 1.2 }
}

fn call_reinforcements(ctx) {
    log_info(`${ctx.entity_id}: Calling for reinforcements!`);

    // Spawn reinforcements
    let pos = ctx.position ?? #{ x: 0.0, y: 0.0, z: 0.0 };

    spawn_npc(#{
        class: ctx.local_data.reinforcement_class ?? "fighter",
        position: #{ x: pos.x + 100.0, y: pos.y, z: pos.z },
        faction: ctx.faction ?? "enemy",
        behavior: "ai/pirate_bt.rhai"
    });

    spawn_npc(#{
        class: ctx.local_data.reinforcement_class ?? "fighter",
        position: #{ x: pos.x - 100.0, y: pos.y, z: pos.z },
        faction: ctx.faction ?? "enemy",
        behavior: "ai/pirate_bt.rhai"
    });

    ctx.local_data.reinforcements_arrived = true;

    emit_event("reinforcements_called", #{
        caller: ctx.entity_id,
        count: 2
    });

    #{ status: "success", action: "call_reinforcements" }
}

fn focused_fire(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id}: Focused fire on ${target}!`);

    #{ status: "success", action: "focused_fire", target: target }
}

fn power_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id}: Power attack!`);

    // Use extra energy for damage boost
    ctx.local_data.energy_used = 30.0;

    #{ status: "success", action: "power_attack", target: target, damage_multiplier: 1.3 }
}

fn standard_attack(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    #{ status: "success", action: "attack", target: target }
}

fn flanking_maneuver(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id}: Flanking maneuver!`);

    #{ status: "success", action: "flank", target: target }
}

fn probe_defenses(ctx) {
    let target = ctx.target ?? "";
    if target == "" { return "failure"; }

    log_info(`${ctx.entity_id}: Probing defenses...`);

    // Light attack to test response
    #{ status: "success", action: "probe", target: target, damage_multiplier: 0.5 }
}

fn engage_target(ctx) {
    let target = ctx.target ?? "";
    if target == "" {
        return #{ status: "running", action: "searching" };
    }

    // Move to phase 2 when engaging
    if ctx.local_data.combat_phase < 2 {
        ctx.local_data.combat_phase = 2;
        log_info(`${ctx.entity_id} engaging target, phase: ENGAGED`);
    }

    #{ status: "success", action: "engage", target: target }
}

// =============================================================================
// LIFECYCLE HOOKS
// =============================================================================

fn on_spawn(ctx) {
    log_info(`Escalating combat AI spawning for ${ctx.entity_id}`);

    // Build behavior tree
    ctx.local_data.behavior_tree = build_behavior_tree();

    // Initialize combat state
    ctx.local_data.combat_phase = 1;  // Start in PROBING
    ctx.local_data.combat_start_time = ctx.game_time;
    ctx.local_data.reinforcements_requested = false;
    ctx.local_data.reinforcements_arrived = false;
    ctx.local_data.reinforcement_class = "pirate_raider";

    // Set up property watches
    setup_watches(ctx);

    ctx
}

fn on_update(ctx) {
    let phase = ctx.local_data.combat_phase ?? 1;
    let combat_time = ctx.game_time - (ctx.local_data.combat_start_time ?? ctx.game_time);

    // Time-based escalation (if combat drags on)
    if combat_time > 60.0 && phase < 2 {
        ctx.local_data.combat_phase = 2;
        log_info(`Combat timer: Escalating to ENGAGED after ${combat_time}s`);
    }

    if combat_time > 120.0 && phase < 3 {
        ctx.local_data.combat_phase = 3;
        log_info(`Combat timer: Escalating to AGGRESSIVE after ${combat_time}s`);
    }

    // Watch target's behavior and adapt
    let target = ctx.target ?? "";
    if target != "" {
        // If target is running, pursue aggressively
        let target_fleeing = ctx.target_status ?? "" == "fleeing";
        if target_fleeing && phase < 3 {
            ctx.local_data.combat_phase = 3;
            log_info("Target fleeing! Pursuing aggressively!");
        }
    }

    ctx
}

fn on_destroy(ctx) {
    // Clean up watches
    unwatch_all_for_entity(ctx.entity_id);

    log_info(`Escalating combat AI destroyed for ${ctx.entity_id}`);
}

// =============================================================================
// EVENT HANDLERS
// =============================================================================

/// Handle being damaged by a specific weapon type
fn on_event(ctx, event) {
    if event.event_type == "damage_received" {
        let damage_type = event.damage_type ?? "kinetic";
        let amount = event.amount ?? 0.0;

        // Big hits cause immediate escalation
        if amount > 30.0 && ctx.local_data.combat_phase < 3 {
            ctx.local_data.combat_phase = 3;
            log_info(`Heavy damage received! Escalating to AGGRESSIVE!`);
        }

        // Track damage types to adapt
        let damage_log = ctx.local_data.damage_log ?? [];
        damage_log.push(damage_type);
        ctx.local_data.damage_log = damage_log;
    }

    ctx
}
