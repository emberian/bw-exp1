// =============================================================================
// Faction AI Director
// =============================================================================
// High-level AI that manages faction-wide strategic decisions.
// Demonstrates: Behavior Trees + Utility AI for strategic planning
//
// Features:
// - Manages faction resources and territory
// - Decides when to expand, defend, or retreat
// - Coordinates multiple ships/fleets
// - Responds to threats dynamically
// - Maintains diplomatic relationships

// =============================================================================
// FACTION STATE
// =============================================================================

/// Faction state structure
fn create_faction_state() {
    #{
        // Resources
        credits: 10000,
        ships: 5,
        stations: 2,

        // Territory
        controlled_sectors: [],
        contested_sectors: [],

        // Military
        military_strength: 50.0,
        threat_level: 0.0,

        // Diplomacy
        relations: #{},  // faction_id -> -100 to 100

        // Strategy
        current_strategy: "balanced",
        expansion_target: "",
        defense_priority: "",
    }
}

// =============================================================================
// BEHAVIOR TREE
// =============================================================================

fn build_behavior_tree() {
    bt_selector([
        // Priority 1: Critical defense - territory under attack
        bt_sequence([
            bt_condition("territory_under_attack"),
            bt_utility([
                #{
                    action: "full_defense",
                    considerations: ["threat_severity", "military_readiness", "territory_value"],
                    weight: 1.2
                },
                #{
                    action: "tactical_retreat",
                    considerations: ["overwhelming_odds", "escape_routes", "territory_expendable"],
                    weight: 0.8
                },
                #{
                    action: "call_allies",
                    considerations: ["alliance_strength", "ally_proximity"],
                    weight: 0.9
                }
            ])
        ]),

        // Priority 2: Opportunity - weak neighbor
        bt_sequence([
            bt_condition("expansion_opportunity"),
            bt_utility([
                #{
                    action: "launch_expansion",
                    considerations: ["target_weakness", "own_strength", "strategic_value"],
                    weight: 1.0
                },
                #{
                    action: "economic_pressure",
                    considerations: ["economic_advantage", "diplomatic_cover"],
                    weight: 0.7
                },
                #{
                    action: "wait_for_opening",
                    considerations: ["patience_value", "risk_aversion"],
                    weight: 0.5
                }
            ])
        ]),

        // Priority 3: Resource management
        bt_sequence([
            bt_condition("resources_critical"),
            bt_selector([
                bt_sequence([
                    bt_condition("can_trade"),
                    bt_action("establish_trade_routes")
                ]),
                bt_sequence([
                    bt_condition("can_raid"),
                    bt_action("launch_raids")
                ]),
                bt_action("consolidate_resources")
            ])
        ]),

        // Priority 4: Diplomatic opportunities
        bt_sequence([
            bt_condition("diplomatic_opportunity"),
            bt_utility([
                #{
                    action: "form_alliance",
                    considerations: ["mutual_benefit", "trust_level", "common_enemy"],
                    weight: 1.0
                },
                #{
                    action: "negotiate_treaty",
                    considerations: ["negotiation_leverage", "peace_value"],
                    weight: 0.8
                },
                #{
                    action: "maintain_neutrality",
                    considerations: ["independence_value", "risk_aversion"],
                    weight: 0.6
                }
            ])
        ]),

        // Priority 5: Default - patrol and maintain
        bt_action("maintain_status_quo")
    ])
}

// =============================================================================
// CONDITIONS
// =============================================================================

fn territory_under_attack(ctx) {
    let contested = ctx.local_data.faction_state.contested_sectors ?? [];
    let threat = ctx.local_data.faction_state.threat_level ?? 0.0;

    contested.len() > 0 || threat > 50.0
}

fn expansion_opportunity(ctx) {
    let state = ctx.local_data.faction_state;
    let strength = state.military_strength ?? 0.0;
    let threat = state.threat_level ?? 0.0;

    // Expand when strong and not threatened
    strength > 60.0 && threat < 30.0
}

fn resources_critical(ctx) {
    let state = ctx.local_data.faction_state;
    let credits = state.credits ?? 0;
    let ships = state.ships ?? 0;

    credits < 3000 || ships < 3
}

fn diplomatic_opportunity(ctx) {
    let state = ctx.local_data.faction_state;
    let relations = state.relations ?? #{};

    // Check if any faction has neutral-ish relations
    for faction in relations.keys() {
        let rel = relations[faction];
        if rel > -20 && rel < 50 {
            return true;  // Opportunity to improve or leverage
        }
    }
    false
}

fn can_trade(ctx) {
    let state = ctx.local_data.faction_state;
    let stations = state.stations ?? 0;
    stations >= 1
}

fn can_raid(ctx) {
    let state = ctx.local_data.faction_state;
    let ships = state.ships ?? 0;
    let strength = state.military_strength ?? 0.0;

    ships >= 2 && strength > 30.0
}

// =============================================================================
// UTILITY CONSIDERATIONS (0.0 - 1.0)
// =============================================================================

// === Defense Considerations ===

fn threat_severity(ctx) {
    let threat = ctx.local_data.faction_state.threat_level ?? 0.0;
    curve_quadratic(clamp(threat / 100.0, 0.0, 1.0))
}

fn military_readiness(ctx) {
    let strength = ctx.local_data.faction_state.military_strength ?? 0.0;
    let ships = ctx.local_data.faction_state.ships ?? 0;

    let strength_score = clamp(strength / 100.0, 0.0, 1.0);
    let ship_score = clamp(ships / 10.0, 0.0, 1.0);

    (strength_score + ship_score) / 2.0
}

fn territory_value(ctx) {
    let controlled = ctx.local_data.faction_state.controlled_sectors ?? [];
    let contested = ctx.local_data.faction_state.contested_sectors ?? [];

    // Contested territory is valuable if we control little else
    if controlled.len() <= 2 {
        0.9  // Can't afford to lose any
    } else {
        0.5  // Can sacrifice if needed
    }
}

fn overwhelming_odds(ctx) {
    let threat = ctx.local_data.faction_state.threat_level ?? 0.0;
    let strength = ctx.local_data.faction_state.military_strength ?? 0.0;

    if strength < 1.0 { return 1.0; }

    let ratio = threat / strength;
    curve_logistic(clamp(ratio / 3.0, 0.0, 1.0))
}

fn escape_routes(ctx) {
    let controlled = ctx.local_data.faction_state.controlled_sectors ?? [];
    clamp(controlled.len() / 5.0, 0.2, 1.0)
}

fn territory_expendable(ctx) {
    1.0 - territory_value(ctx)
}

fn alliance_strength(ctx) {
    let relations = ctx.local_data.faction_state.relations ?? #{};
    let allied_count = 0;

    for faction in relations.keys() {
        if relations[faction] > 50 {
            allied_count += 1;
        }
    }

    clamp(allied_count / 3.0, 0.0, 1.0)
}

fn ally_proximity(ctx) {
    // Simplified - would check actual distances
    0.5
}

// === Expansion Considerations ===

fn target_weakness(ctx) {
    let target_strength = ctx.local_data.expansion_target_strength ?? 50.0;
    curve_inverse_quadratic(1.0 - clamp(target_strength / 100.0, 0.0, 1.0))
}

fn own_strength(ctx) {
    let strength = ctx.local_data.faction_state.military_strength ?? 0.0;
    curve_quadratic(clamp(strength / 100.0, 0.0, 1.0))
}

fn strategic_value(ctx) {
    // How valuable is the target territory?
    ctx.local_data.expansion_target_value ?? 0.5
}

fn economic_advantage(ctx) {
    let credits = ctx.local_data.faction_state.credits ?? 0;
    curve_logistic(clamp(credits / 20000.0, 0.0, 1.0))
}

fn diplomatic_cover(ctx) {
    // Do we have diplomatic justification?
    let relations = ctx.local_data.faction_state.relations ?? #{};
    let target = ctx.local_data.expansion_target_faction ?? "";

    if target != "" {
        let rel = relations[target] ?? 0;
        if rel < -50 { return 0.9; }  // They're our enemy anyway
        if rel < 0 { return 0.5; }    // Strained relations
    }
    0.2  // No good excuse
}

fn patience_value(ctx) {
    // Sometimes waiting is the right move
    let threat = ctx.local_data.faction_state.threat_level ?? 0.0;
    if threat > 30.0 { return 0.8; }  // Wait if threatened
    0.3
}

fn risk_aversion(ctx) {
    let ships = ctx.local_data.faction_state.ships ?? 0;
    let credits = ctx.local_data.faction_state.credits ?? 0;

    // More risk-averse when resources are low
    if ships < 3 || credits < 5000 {
        0.8
    } else {
        0.3
    }
}

// === Diplomacy Considerations ===

fn mutual_benefit(ctx) {
    0.6  // Baseline - would calculate based on specific faction
}

fn trust_level(ctx) {
    let target = ctx.local_data.diplomacy_target ?? "";
    let relations = ctx.local_data.faction_state.relations ?? #{};

    if target != "" {
        let rel = relations[target] ?? 0;
        return clamp((rel + 100.0) / 200.0, 0.0, 1.0);
    }
    0.5
}

fn common_enemy(ctx) {
    // Do we share enemies with this faction?
    0.5  // Simplified
}

fn negotiation_leverage(ctx) {
    let strength = ctx.local_data.faction_state.military_strength ?? 0.0;
    let credits = ctx.local_data.faction_state.credits ?? 0;

    (clamp(strength / 100.0, 0.0, 1.0) + clamp(credits / 20000.0, 0.0, 1.0)) / 2.0
}

fn peace_value(ctx) {
    let threat = ctx.local_data.faction_state.threat_level ?? 0.0;

    // Peace is more valuable when threatened
    curve_quadratic(clamp(threat / 100.0, 0.0, 1.0))
}

fn independence_value(ctx) {
    // How much do we value going it alone?
    let controlled = ctx.local_data.faction_state.controlled_sectors ?? [];

    if controlled.len() >= 5 {
        0.7  // Strong enough to be independent
    } else {
        0.3  // Might need allies
    }
}

// =============================================================================
// ACTIONS
// =============================================================================

fn full_defense(ctx) {
    log_info("Faction Director: Initiating FULL DEFENSE protocol");

    let contested = ctx.local_data.faction_state.contested_sectors ?? [];

    // Order all ships to defend
    emit_event("faction_order", #{
        order: "defend",
        targets: contested,
        priority: "maximum"
    });

    ctx.local_data.faction_state.current_strategy = "defensive";

    #{ status: "success", action: "full_defense", sectors: contested }
}

fn tactical_retreat(ctx) {
    log_info("Faction Director: Ordering TACTICAL RETREAT");

    let contested = ctx.local_data.faction_state.contested_sectors ?? [];

    emit_event("faction_order", #{
        order: "retreat",
        from_sectors: contested,
        priority: "high"
    });

    // Cede contested territory
    ctx.local_data.faction_state.contested_sectors = [];

    #{ status: "success", action: "retreat" }
}

fn call_allies(ctx) {
    log_info("Faction Director: Calling for allied support");

    emit_event("diplomatic_request", #{
        request_type: "military_aid",
        urgency: "high"
    });

    #{ status: "success", action: "call_allies" }
}

fn launch_expansion(ctx) {
    let target = ctx.local_data.expansion_target ?? "";
    log_info(`Faction Director: Launching EXPANSION to ${target}`);

    emit_event("faction_order", #{
        order: "attack",
        target_sector: target,
        priority: "high"
    });

    ctx.local_data.faction_state.current_strategy = "expansion";

    #{ status: "success", action: "expand", target: target }
}

fn economic_pressure(ctx) {
    log_info("Faction Director: Applying economic pressure");

    emit_event("economic_action", #{
        action: "embargo",
        target: ctx.local_data.expansion_target_faction ?? ""
    });

    #{ status: "success", action: "economic_pressure" }
}

fn wait_for_opening(ctx) {
    log_info("Faction Director: Waiting for better opportunity");
    #{ status: "success", action: "wait" }
}

fn establish_trade_routes(ctx) {
    log_info("Faction Director: Establishing trade routes");

    emit_event("economic_action", #{
        action: "open_trade",
        all_stations: true
    });

    #{ status: "success", action: "trade" }
}

fn launch_raids(ctx) {
    log_info("Faction Director: Launching resource raids");

    emit_event("faction_order", #{
        order: "raid",
        target_type: "cargo_ships",
        priority: "medium"
    });

    #{ status: "success", action: "raid" }
}

fn consolidate_resources(ctx) {
    log_info("Faction Director: Consolidating resources");

    emit_event("economic_action", #{
        action: "consolidate",
        reduce_patrols: true
    });

    #{ status: "success", action: "consolidate" }
}

fn form_alliance(ctx) {
    let target = ctx.local_data.diplomacy_target ?? "";
    log_info(`Faction Director: Proposing alliance with ${target}`);

    emit_event("diplomatic_action", #{
        action: "propose_alliance",
        target_faction: target
    });

    #{ status: "success", action: "alliance_proposal", target: target }
}

fn negotiate_treaty(ctx) {
    let target = ctx.local_data.diplomacy_target ?? "";
    log_info(`Faction Director: Negotiating treaty with ${target}`);

    emit_event("diplomatic_action", #{
        action: "negotiate_treaty",
        target_faction: target
    });

    #{ status: "success", action: "treaty_negotiation", target: target }
}

fn maintain_neutrality(ctx) {
    log_info("Faction Director: Maintaining neutral stance");
    #{ status: "success", action: "neutral" }
}

fn maintain_status_quo(ctx) {
    // Default patrol and maintenance
    emit_event("faction_order", #{
        order: "patrol",
        priority: "low"
    });

    #{ status: "running", action: "maintain" }
}

// =============================================================================
// LIFECYCLE HOOKS
// =============================================================================

fn on_spawn(ctx) {
    log_info(`Faction Director initializing for faction ${ctx.entity_id}`);

    // Build the decision tree
    ctx.local_data.behavior_tree = build_behavior_tree();

    // Initialize faction state
    ctx.local_data.faction_state = create_faction_state();

    // Load persisted state if available
    let saved = load_state(`faction:${ctx.entity_id}`);
    if saved != () {
        ctx.local_data.faction_state = saved;
        log_info("Loaded faction state from persistence");
    }

    ctx
}

fn on_update(ctx) {
    // Update threat assessment
    let threats = ctx.nearby_hostile_count ?? 0;
    let strength = ctx.local_data.faction_state.military_strength ?? 50.0;

    ctx.local_data.faction_state.threat_level = threats * 20.0;

    // Identify expansion opportunities
    let weak_neighbors = ctx.weak_neighbor_sectors ?? [];
    if weak_neighbors.len() > 0 {
        ctx.local_data.expansion_target = weak_neighbors[0];
        ctx.local_data.expansion_target_strength = ctx.neighbor_strength ?? 30.0;
        ctx.local_data.expansion_target_value = 0.6;
    }

    // Identify diplomatic opportunities
    let neutral_factions = ctx.neutral_factions ?? [];
    if neutral_factions.len() > 0 {
        ctx.local_data.diplomacy_target = neutral_factions[0];
    }

    ctx
}

fn on_destroy(ctx) {
    // Save faction state
    save_state(`faction:${ctx.entity_id}`, ctx.local_data.faction_state);
    log_info(`Faction Director shutting down, state saved`);
}
