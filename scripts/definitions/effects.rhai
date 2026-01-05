// =============================================================================
// Effect Definitions
// =============================================================================
// All combat effects are defined here. Effects are applied by weapons,
// abilities, or other game systems.
//
// Required function: all_effects() - returns array of all effect definitions
//
// Effect Types:
// - instant: Applied immediately, no duration
// - duration: Lasts for multiple ticks
// - trigger: Activates on specific events

// =============================================================================
// Damage Modifier Effects
// =============================================================================

fn shield_piercing() {
    #{
        id: "shield_piercing",
        name: "Shield Piercing",
        description: "Bypasses a portion of target shields.",
        type: "instant",
        handler: "shield_pierce",
        params: #{
            bypass_percent: 0.5,  // 50% of damage bypasses shields
        },
        stacking: "ignore",  // Multiple applications don't stack
        trigger: (),
        icon: "shield_break",
    }
}

fn armor_piercing() {
    #{
        id: "armor_piercing",
        name: "Armor Piercing",
        description: "Ignores a portion of target defense.",
        type: "instant",
        handler: "armor_pierce",
        params: #{
            defense_ignore_percent: 0.35,  // Ignore 35% of defense
        },
        stacking: "ignore",
        trigger: (),
        icon: "armor_break",
    }
}

fn critical_boost() {
    #{
        id: "critical_boost",
        name: "Critical Boost",
        description: "Increases critical hit chance.",
        type: "instant",
        handler: "crit_modifier",
        params: #{
            crit_bonus: 0.15,  // +15% crit chance
        },
        stacking: "stack",
        trigger: (),
        icon: "critical",
    }
}

// =============================================================================
// Damage Over Time Effects
// =============================================================================

fn burn() {
    #{
        id: "burn",
        name: "Burn",
        description: "Target takes damage over time from heat.",
        type: "duration",
        handler: "dot",
        params: #{
            damage_per_tick: 3.0,
            duration_ticks: 30,
            damage_type: "thermal",
        },
        stacking: "refresh",  // Resets duration, doesn't stack damage
        trigger: (),
        icon: "fire",
    }
}

fn corrosion() {
    #{
        id: "corrosion",
        name: "Corrosion",
        description: "Acid eats through armor, dealing damage and reducing defense.",
        type: "duration",
        handler: "dot",
        params: #{
            damage_per_tick: 2.0,
            duration_ticks: 50,
            damage_type: "acid",
            defense_reduction: 0.1,  // -10% defense while active
        },
        stacking: "stack",  // Multiple applications stack
        trigger: (),
        icon: "acid",
    }
}

fn bleed() {
    #{
        id: "bleed",
        name: "Bleed",
        description: "Internal damage causes ongoing hull damage.",
        type: "duration",
        handler: "dot",
        params: #{
            damage_per_tick: 4.0,
            duration_ticks: 20,
            damage_type: "physical",
            bypass_shields: true,  // Bleeds ignore shields
        },
        stacking: "stack",
        trigger: (),
        icon: "bleed",
    }
}

// =============================================================================
// Disable/Control Effects
// =============================================================================

fn emp() {
    #{
        id: "emp",
        name: "EMP",
        description: "Electromagnetic pulse disables ship systems.",
        type: "duration",
        handler: "emp",
        params: #{
            disable_duration: 15,
            systems: ["shields", "weapons"],
            disable_chance: 0.8,
        },
        stacking: "refresh",
        trigger: (),
        icon: "lightning",
    }
}

fn slow() {
    #{
        id: "slow",
        name: "Slow",
        description: "Reduces target speed and maneuverability.",
        type: "duration",
        handler: "stat_modifier",
        params: #{
            stat: "speed",
            modifier: -0.4,  // -40% speed
            duration_ticks: 40,
        },
        stacking: "ignore",  // Only one slow active at a time
        trigger: (),
        icon: "snail",
    }
}

fn disable() {
    #{
        id: "disable",
        name: "Disable",
        description: "Completely disables target for a short time.",
        type: "duration",
        handler: "emp",
        params: #{
            disable_duration: 8,
            systems: ["all"],
            disable_chance: 1.0,
        },
        stacking: "refresh",
        trigger: (),
        icon: "power_off",
    }
}

fn tractor() {
    #{
        id: "tractor",
        name: "Tractor Lock",
        description: "Locks target in place with gravitational beam.",
        type: "duration",
        handler: "tractor",
        params: #{
            duration_ticks: 30,
            pull_strength: 0.5,
            break_threshold: 100.0,  // Damage to break free
        },
        stacking: "ignore",
        trigger: (),
        icon: "tractor",
    }
}

// =============================================================================
// Stat Modifier Effects
// =============================================================================

fn attack_boost() {
    #{
        id: "attack_boost",
        name: "Attack Boost",
        description: "Temporarily increases attack damage.",
        type: "duration",
        handler: "stat_modifier",
        params: #{
            stat: "attack",
            modifier: 0.25,  // +25% attack
            duration_ticks: 60,
        },
        stacking: "refresh",
        trigger: (),
        icon: "sword_up",
    }
}

fn defense_boost() {
    #{
        id: "defense_boost",
        name: "Defense Boost",
        description: "Temporarily increases defense.",
        type: "duration",
        handler: "stat_modifier",
        params: #{
            stat: "defense",
            modifier: 0.30,  // +30% defense
            duration_ticks: 60,
        },
        stacking: "refresh",
        trigger: (),
        icon: "shield_up",
    }
}

fn speed_boost() {
    #{
        id: "speed_boost",
        name: "Speed Boost",
        description: "Temporarily increases speed.",
        type: "duration",
        handler: "stat_modifier",
        params: #{
            stat: "speed",
            modifier: 0.50,  // +50% speed
            duration_ticks: 40,
        },
        stacking: "refresh",
        trigger: (),
        icon: "rocket",
    }
}

fn accuracy_boost() {
    #{
        id: "accuracy_boost",
        name: "Targeting Lock",
        description: "Increases accuracy against target.",
        type: "duration",
        handler: "stat_modifier",
        params: #{
            stat: "accuracy",
            modifier: 0.20,  // +20% accuracy
            duration_ticks: 50,
        },
        stacking: "refresh",
        trigger: (),
        icon: "crosshair",
    }
}

// =============================================================================
// Shield Effects
// =============================================================================

fn shield_restore() {
    #{
        id: "shield_restore",
        name: "Shield Restore",
        description: "Instantly restores shield points.",
        type: "instant",
        handler: "heal",
        params: #{
            amount: 50.0,
            heal_type: "shields",
        },
        stacking: "stack",
        trigger: (),
        icon: "shield_plus",
    }
}

fn shield_regen() {
    #{
        id: "shield_regen",
        name: "Shield Regeneration",
        description: "Shields regenerate over time.",
        type: "duration",
        handler: "hot",  // Heal over time
        params: #{
            heal_per_tick: 2.0,
            duration_ticks: 50,
            heal_type: "shields",
        },
        stacking: "refresh",
        trigger: (),
        icon: "shield_regen",
    }
}

fn shield_overcharge() {
    #{
        id: "shield_overcharge",
        name: "Shield Overcharge",
        description: "Temporarily increases maximum shields.",
        type: "duration",
        handler: "max_stat_modifier",
        params: #{
            stat: "shield_capacity",
            modifier: 0.5,  // +50% max shields
            duration_ticks: 80,
            fill_bonus: true,  // Also fills the extra capacity
        },
        stacking: "ignore",
        trigger: (),
        icon: "shield_max",
    }
}

// =============================================================================
// Hull Effects
// =============================================================================

fn hull_repair() {
    #{
        id: "hull_repair",
        name: "Hull Repair",
        description: "Instantly repairs hull damage.",
        type: "instant",
        handler: "heal",
        params: #{
            amount: 30.0,
            heal_type: "hull",
        },
        stacking: "stack",
        trigger: (),
        icon: "wrench",
    }
}

fn hull_regen() {
    #{
        id: "hull_regen",
        name: "Hull Regeneration",
        description: "Hull slowly repairs over time.",
        type: "duration",
        handler: "hot",
        params: #{
            heal_per_tick: 1.0,
            duration_ticks: 60,
            heal_type: "hull",
        },
        stacking: "refresh",
        trigger: (),
        icon: "hull_regen",
    }
}

// =============================================================================
// Weapon Effects
// =============================================================================

fn rapid_fire() {
    #{
        id: "rapid_fire",
        name: "Rapid Fire",
        description: "Weapon fires multiple shots.",
        type: "instant",
        handler: "multi_attack",
        params: #{
            extra_attacks: 2,
            damage_mult: 0.5,  // Each shot does 50% damage
        },
        stacking: "ignore",
        trigger: (),
        icon: "bullets",
    }
}

fn explosive() {
    #{
        id: "explosive",
        name: "Explosive",
        description: "Weapon deals area damage on impact.",
        type: "instant",
        handler: "aoe",
        params: #{
            radius: 50.0,
            falloff: 0.5,  // 50% damage reduction per radius
            friendly_fire: false,
        },
        stacking: "stack",
        trigger: (),
        icon: "explosion",
    }
}

fn tracking() {
    #{
        id: "tracking",
        name: "Tracking",
        description: "Projectile tracks target, improving accuracy.",
        type: "instant",
        handler: "accuracy_modifier",
        params: #{
            accuracy_bonus: 0.15,
            evasion_ignore: 0.2,  // Ignores 20% of target evasion
        },
        stacking: "ignore",
        trigger: (),
        icon: "target_lock",
    }
}

fn swarm() {
    #{
        id: "swarm",
        name: "Swarm",
        description: "Multiple projectiles overwhelm point defense.",
        type: "instant",
        handler: "multi_attack",
        params: #{
            extra_attacks: 4,
            damage_mult: 0.3,
            point_defense_penalty: 0.5,  // Harder to shoot down
        },
        stacking: "ignore",
        trigger: (),
        icon: "swarm",
    }
}

fn anti_missile() {
    #{
        id: "anti_missile",
        name: "Anti-Missile",
        description: "Can intercept incoming missiles.",
        type: "trigger",
        handler: "intercept",
        params: #{
            intercept_chance: 0.6,
            targets: ["missile", "torpedo"],
        },
        stacking: "ignore",
        trigger: "on_incoming_missile",
        icon: "intercept",
    }
}

fn area_damage() {
    #{
        id: "area_damage",
        name: "Area Damage",
        description: "Weapon damages all targets in area.",
        type: "instant",
        handler: "aoe",
        params: #{
            radius: 80.0,
            falloff: 0.3,
            friendly_fire: false,
        },
        stacking: "ignore",
        trigger: (),
        icon: "area",
    }
}

// =============================================================================
// Special Effects
// =============================================================================

fn mining() {
    #{
        id: "mining",
        name: "Mining",
        description: "Can extract resources from asteroids.",
        type: "instant",
        handler: "mining",
        params: #{
            yield_mult: 1.0,
            asteroid_damage_mult: 3.0,
        },
        stacking: "ignore",
        trigger: (),
        icon: "pickaxe",
    }
}

fn organic() {
    #{
        id: "organic",
        name: "Organic",
        description: "Biological weapon, regenerates ammunition.",
        type: "trigger",
        handler: "ammo_regen",
        params: #{
            regen_per_tick: 0.1,
        },
        stacking: "ignore",
        trigger: "on_tick",
        icon: "bio",
    }
}

fn regenerating() {
    #{
        id: "regenerating",
        name: "Regenerating",
        description: "Weapon regenerates ammunition over time.",
        type: "trigger",
        handler: "ammo_regen",
        params: #{
            regen_per_tick: 0.05,
        },
        stacking: "ignore",
        trigger: "on_tick",
        icon: "regen",
    }
}

fn acid() {
    #{
        id: "acid",
        name: "Acid",
        description: "Corrosive damage that eats through armor.",
        type: "instant",
        handler: "apply_effect",
        params: #{
            effect_id: "corrosion",
        },
        stacking: "stack",
        trigger: (),
        icon: "acid",
    }
}

fn single_use() {
    #{
        id: "single_use",
        name: "Single Use",
        description: "Weapon can only fire once.",
        type: "trigger",
        handler: "single_use",
        params: #{
            destroy_after_use: true,
        },
        stacking: "ignore",
        trigger: "on_attack",
        icon: "warning",
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all effect definitions.
/// This function is called by the ArchetypeRegistry to load effects.
fn all_effects() {
    [
        // Damage modifiers
        shield_piercing(),
        armor_piercing(),
        critical_boost(),

        // Damage over time
        burn(),
        corrosion(),
        bleed(),

        // Disable/control
        emp(),
        slow(),
        disable(),
        tractor(),

        // Stat modifiers
        attack_boost(),
        defense_boost(),
        speed_boost(),
        accuracy_boost(),

        // Shield effects
        shield_restore(),
        shield_regen(),
        shield_overcharge(),

        // Hull effects
        hull_repair(),
        hull_regen(),

        // Weapon effects
        rapid_fire(),
        explosive(),
        tracking(),
        swarm(),
        anti_missile(),
        area_damage(),

        // Special effects
        mining(),
        organic(),
        regenerating(),
        acid(),
        single_use(),
    ]
}
