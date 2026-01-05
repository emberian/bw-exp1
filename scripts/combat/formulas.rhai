// Combat Formulas Script
// All damage, accuracy, and combat calculations.
// Modders can adjust these to rebalance combat.

// ============================================================================
// DAMAGE CALCULATIONS
// ============================================================================

/// Calculate base weapon damage
fn calculate_weapon_damage(weapon, attacker) {
    let base = weapon["base_damage"];
    let attack_stat = attacker["attack"];

    // Attack stat provides percentage bonus (100 attack = +100% damage)
    let attack_mult = 1.0 + (attack_stat / 100.0);

    base * attack_mult
}

/// Calculate damage after defenses
fn calculate_final_damage(raw_damage, defender) {
    let defense = defender["defense"];
    let shields = defender["shields"];

    // Defense provides damage reduction (caps at 50%)
    let defense_reduction = (defense / 200.0).min(0.5);
    let after_defense = raw_damage * (1.0 - defense_reduction);

    // Shields absorb damage first
    let shield_absorbed = shields.min(after_defense);
    let hull_damage = after_defense - shield_absorbed;

    #{
        shield_damage: shield_absorbed,
        hull_damage: hull_damage,
        total_damage: after_defense
    }
}

/// Calculate critical hit
fn check_critical(attacker, weapon) {
    let base_crit = weapon["crit_chance"];
    let experience = attacker["experience"];

    // Experience adds to crit chance (caps at +15%)
    let exp_bonus = (experience / 1000.0).min(0.15);
    let crit_chance = base_crit + exp_bonus;

    if rand() < crit_chance {
        // Critical multiplier (base 1.5x, can be modified by weapon)
        let crit_mult = weapon["crit_multiplier"];
        if crit_mult == () { crit_mult = 1.5; }

        return #{ critical: true, multiplier: crit_mult };
    }

    #{ critical: false, multiplier: 1.0 }
}

// ============================================================================
// ACCURACY CALCULATIONS
// ============================================================================

/// Calculate hit chance
fn calculate_hit_chance(attacker, defender, weapon) {
    let base_accuracy = weapon["accuracy"];
    let attacker_speed = attacker["speed"];
    let defender_speed = defender["speed"];
    let distance = calculate_distance(attacker, defender);

    // Speed differential affects accuracy
    let speed_mod = attacker_speed / defender_speed.max(1.0);

    // Distance penalty (weapons have optimal range)
    let optimal_range = weapon["optimal_range"];
    if optimal_range == () { optimal_range = 100.0; }

    let range_mod = if distance <= optimal_range {
        1.0
    } else {
        let over_range = distance - optimal_range;
        1.0 - (over_range / optimal_range).min(0.5)
    };

    // Morale affects accuracy
    let morale = attacker["morale"];
    let morale_mod = if morale > 50.0 {
        1.0 + ((morale - 50.0) / 100.0)  // Up to +50% at 100 morale
    } else {
        0.5 + (morale / 100.0)  // Down to 50% at 0 morale
    };

    let final_accuracy = base_accuracy * speed_mod * range_mod * morale_mod;

    // Clamp between 10% and 95%
    clamp(final_accuracy, 0.10, 0.95)
}

/// Calculate evasion
fn calculate_evasion(defender) {
    let speed = defender["speed"];
    let maneuverability = defender["maneuverability"];

    // Base evasion from maneuverability
    let base_evasion = maneuverability * 0.05; // 5% per point

    // Speed bonus
    let speed_bonus = (speed / 100.0) * 0.1; // Up to 10% from speed

    clamp(base_evasion + speed_bonus, 0.0, 0.40) // Cap at 40%
}

// ============================================================================
// SPECIAL DAMAGE TYPES
// ============================================================================

/// Calculate shield-piercing damage
fn calculate_piercing_damage(raw_damage, weapon, defender) {
    let pierce_percent = weapon["shield_pierce"];
    if pierce_percent == () { pierce_percent = 0.0; }

    let shields = defender["shields"];
    let defense = defender["defense"];

    // Defense reduction
    let defense_reduction = (defense / 200.0).min(0.5);
    let after_defense = raw_damage * (1.0 - defense_reduction);

    // Split damage: some goes through shields
    let piercing_damage = after_defense * pierce_percent;
    let normal_damage = after_defense * (1.0 - pierce_percent);

    // Normal damage absorbed by shields
    let shield_absorbed = shields.min(normal_damage);
    let hull_from_normal = normal_damage - shield_absorbed;

    #{
        shield_damage: shield_absorbed,
        hull_damage: hull_from_normal + piercing_damage,
        piercing_damage: piercing_damage
    }
}

/// Calculate area-of-effect damage
fn calculate_aoe_damage(raw_damage, weapon, targets) {
    let aoe_radius = weapon["aoe_radius"];
    let falloff = weapon["aoe_falloff"];
    if falloff == () { falloff = 0.5; } // 50% falloff per radius

    let source_x = weapon["source_x"];
    let source_y = weapon["source_y"];

    let results = [];

    for target in targets {
        let dx = target["position_x"] - source_x;
        let dy = target["position_y"] - source_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= aoe_radius {
            // Damage falls off with distance
            let distance_factor = 1.0 - (dist / aoe_radius) * falloff;
            let damage = raw_damage * distance_factor;

            results.push(#{
                target: target["id"],
                damage: damage
            });
        }
    }

    results
}

// ============================================================================
// AMMUNITION CONSUMPTION
// ============================================================================

/// Calculate ammo consumed by attack
fn calculate_ammo_consumption(weapon, attacker) {
    let base_cost = weapon["ammo_cost"];
    if base_cost == () { base_cost = 1.0; }

    // Low morale = wasteful (panic fire)
    let morale = attacker["morale"];
    let morale_mod = if morale < 30.0 {
        1.5  // 50% more ammo used
    } else if morale < 50.0 {
        1.2  // 20% more ammo used
    } else {
        1.0
    };

    base_cost * morale_mod
}

// ============================================================================
// COMBAT MODIFIERS
// ============================================================================

/// Calculate morale effects on combat
fn get_morale_combat_modifiers(morale) {
    if morale >= 80.0 {
        #{ damage_mult: 1.15, accuracy_mult: 1.10, defense_mult: 1.05 }
    } else if morale >= 60.0 {
        #{ damage_mult: 1.05, accuracy_mult: 1.05, defense_mult: 1.0 }
    } else if morale >= 40.0 {
        #{ damage_mult: 1.0, accuracy_mult: 1.0, defense_mult: 1.0 }
    } else if morale >= 20.0 {
        #{ damage_mult: 0.90, accuracy_mult: 0.90, defense_mult: 0.95 }
    } else {
        #{ damage_mult: 0.75, accuracy_mult: 0.75, defense_mult: 0.85 }
    }
}

/// Calculate experience effects
fn get_experience_combat_modifiers(experience) {
    // Experience provides small but consistent bonuses
    let exp_factor = (experience / 500.0).min(1.0); // Caps at 500 exp

    #{
        crit_bonus: exp_factor * 0.10,      // Up to +10% crit
        accuracy_bonus: exp_factor * 0.05,  // Up to +5% accuracy
        damage_bonus: exp_factor * 0.10     // Up to +10% damage
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

fn calculate_distance(a, b) {
    let dx = a["position_x"] - b["position_x"];
    let dy = a["position_y"] - b["position_y"];
    (dx * dx + dy * dy).sqrt()
}

/// Full attack resolution
fn resolve_attack(attacker, defender, weapon) {
    // Calculate hit
    let hit_chance = calculate_hit_chance(attacker, defender, weapon);
    let evasion = calculate_evasion(defender);
    let final_hit_chance = hit_chance - evasion;

    if rand() >= final_hit_chance {
        return #{
            hit: false,
            damage: 0.0,
            ammo_used: calculate_ammo_consumption(weapon, attacker),
            message: "Attack missed"
        };
    }

    // Calculate damage
    let base_damage = calculate_weapon_damage(weapon, attacker);

    // Check critical
    let crit_result = check_critical(attacker, weapon);
    let damage_after_crit = base_damage * crit_result["multiplier"];

    // Apply morale modifiers
    let morale_mods = get_morale_combat_modifiers(attacker["morale"]);
    let damage_after_morale = damage_after_crit * morale_mods["damage_mult"];

    // Apply experience modifiers
    let exp_mods = get_experience_combat_modifiers(attacker["experience"]);
    let final_raw_damage = damage_after_morale * (1.0 + exp_mods["damage_bonus"]);

    // Calculate final damage against defenses
    let damage_result = calculate_final_damage(final_raw_damage, defender);

    #{
        hit: true,
        critical: crit_result["critical"],
        shield_damage: damage_result["shield_damage"],
        hull_damage: damage_result["hull_damage"],
        total_damage: damage_result["total_damage"],
        ammo_used: calculate_ammo_consumption(weapon, attacker),
        message: if crit_result["critical"] { "Critical hit!" } else { "Hit" }
    }
}
