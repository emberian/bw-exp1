// =============================================================================
// Ability Definitions
// =============================================================================
// All player abilities are defined here. Abilities are active or passive
// skills that provide combat and utility advantages.
//
// Required function: all_abilities() - returns array of all ability definitions
//
// Ability Targets:
// - self: Affects the user only
// - friendly: Target must be friendly/allied
// - enemy: Target must be hostile
// - any: Can target anyone
// - area: Affects all in radius

// =============================================================================
// Defensive Abilities
// =============================================================================

fn emergency_shields() {
    #{
        id: "emergency_shields",
        name: "Emergency Shields",
        description: "Rapidly restore shields in an emergency.",
        cooldown: 60,
        cost: #{ energy: 25 },
        target: "self",
        effects: [
            #{ effect_id: "shield_restore", params: #{ amount: 50 } },
        ],
        requirements: #{ min_tier: 1 },
        icon: "shield_emergency",
        tier: 1,
        is_passive: false,
    }
}

fn shield_overcharge() {
    #{
        id: "shield_overcharge",
        name: "Shield Overcharge",
        description: "Temporarily boost maximum shield capacity.",
        cooldown: 120,
        cost: #{ energy: 40 },
        target: "self",
        effects: [
            #{ effect_id: "shield_overcharge", params: #{ modifier: 0.5, duration_ticks: 100 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "shield_max",
        tier: 2,
        is_passive: false,
    }
}

fn emergency_repair() {
    #{
        id: "emergency_repair",
        name: "Emergency Repair",
        description: "Quick hull repair during combat.",
        cooldown: 90,
        cost: #{ energy: 30, credits: 100 },
        target: "self",
        effects: [
            #{ effect_id: "hull_repair", params: #{ amount: 25 } },
        ],
        requirements: #{ min_tier: 1 },
        icon: "wrench",
        tier: 1,
        is_passive: false,
    }
}

fn damage_control() {
    #{
        id: "damage_control",
        name: "Damage Control",
        description: "Repair systems and restore hull over time.",
        cooldown: 150,
        cost: #{ energy: 50 },
        target: "self",
        effects: [
            #{ effect_id: "hull_regen", params: #{ heal_per_tick: 2.0, duration_ticks: 80 } },
            #{ effect_id: "shield_regen", params: #{ heal_per_tick: 1.5, duration_ticks: 80 } },
        ],
        requirements: #{ min_tier: 3 },
        icon: "repair_drone",
        tier: 3,
        is_passive: false,
    }
}

// =============================================================================
// Offensive Abilities
// =============================================================================

fn overdrive() {
    #{
        id: "overdrive",
        name: "Overdrive",
        description: "Push weapons systems beyond safe limits.",
        cooldown: 45,
        cost: #{ energy: 40 },
        target: "self",
        effects: [
            #{ effect_id: "attack_boost", params: #{ modifier: 0.5, duration_ticks: 60 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "overdrive",
        tier: 2,
        is_passive: false,
    }
}

fn targeting_computer() {
    #{
        id: "targeting_computer",
        name: "Targeting Computer",
        description: "Activate advanced targeting for improved accuracy.",
        cooldown: 30,
        cost: #{ energy: 15 },
        target: "enemy",
        effects: [
            #{ effect_id: "accuracy_boost", params: #{ modifier: 0.25, duration_ticks: 40 } },
        ],
        requirements: #{ min_tier: 1 },
        icon: "crosshair",
        tier: 1,
        is_passive: false,
    }
}

fn emp_burst() {
    #{
        id: "emp_burst",
        name: "EMP Burst",
        description: "Disable enemy systems with an electromagnetic pulse.",
        cooldown: 90,
        cost: #{ energy: 60, ammunition: 10 },
        target: "enemy",
        effects: [
            #{ effect_id: "emp", params: #{ disable_duration: 20, systems: ["shields", "weapons"] } },
        ],
        requirements: #{ min_tier: 3 },
        icon: "emp",
        tier: 3,
        is_passive: false,
    }
}

fn missile_barrage() {
    #{
        id: "missile_barrage",
        name: "Missile Barrage",
        description: "Launch a devastating volley of missiles.",
        cooldown: 60,
        cost: #{ ammunition: 20 },
        target: "enemy",
        effects: [
            #{ effect_id: "explosive", params: #{ radius: 60, falloff: 0.4 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "missiles",
        tier: 2,
        is_passive: false,
    }
}

fn focused_strike() {
    #{
        id: "focused_strike",
        name: "Focused Strike",
        description: "Concentrate fire on a single target for massive damage.",
        cooldown: 40,
        cost: #{ energy: 35, ammunition: 5 },
        target: "enemy",
        effects: [
            #{ effect_id: "critical_boost", params: #{ crit_bonus: 0.4 } },
            #{ effect_id: "armor_piercing", params: #{ defense_ignore_percent: 0.5 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "focused",
        tier: 2,
        is_passive: false,
    }
}

// =============================================================================
// Movement Abilities
// =============================================================================

fn afterburner() {
    #{
        id: "afterburner",
        name: "Afterburner",
        description: "Boost engines for a burst of speed.",
        cooldown: 20,
        cost: #{ fuel: 5 },
        target: "self",
        effects: [
            #{ effect_id: "speed_boost", params: #{ modifier: 0.8, duration_ticks: 30 } },
        ],
        requirements: #{ min_tier: 1 },
        icon: "afterburner",
        tier: 1,
        is_passive: false,
    }
}

fn emergency_jump() {
    #{
        id: "emergency_jump",
        name: "Emergency Jump",
        description: "Quickly jump to an adjacent sector.",
        cooldown: 180,
        cost: #{ fuel: 20, energy: 50 },
        target: "self",
        effects: [
            // Handled by special handler - teleports to adjacent sector
        ],
        requirements: #{ min_tier: 3 },
        icon: "jump",
        tier: 3,
        is_passive: false,
    }
}

fn evasive_maneuvers() {
    #{
        id: "evasive_maneuvers",
        name: "Evasive Maneuvers",
        description: "Perform evasive maneuvers to avoid attacks.",
        cooldown: 25,
        cost: #{ fuel: 3 },
        target: "self",
        effects: [
            #{ effect_id: "defense_boost", params: #{ modifier: 0.4, duration_ticks: 25 } },
            #{ effect_id: "speed_boost", params: #{ modifier: 0.2, duration_ticks: 25 } },
        ],
        requirements: #{ min_tier: 1 },
        icon: "evasion",
        tier: 1,
        is_passive: false,
    }
}

// =============================================================================
// Utility Abilities
// =============================================================================

fn scan_target() {
    #{
        id: "scan_target",
        name: "Scan Target",
        description: "Reveal detailed information about target ship.",
        cooldown: 15,
        cost: #{ energy: 5 },
        target: "any",
        effects: [
            // Reveals target stats, cargo, abilities
        ],
        requirements: #{ min_tier: 1 },
        icon: "scan",
        tier: 1,
        is_passive: false,
    }
}

fn distress_beacon() {
    #{
        id: "distress_beacon",
        name: "Distress Beacon",
        description: "Call for help from nearby allies.",
        cooldown: 300,
        cost: #{ energy: 10 },
        target: "self",
        effects: [
            // Summons allied patrol if in controlled space
        ],
        requirements: #{ min_tier: 1 },
        icon: "distress",
        tier: 1,
        is_passive: false,
    }
}

fn cloak() {
    #{
        id: "cloak",
        name: "Cloaking Device",
        description: "Become temporarily invisible to sensors.",
        cooldown: 120,
        cost: #{ energy: 80 },
        target: "self",
        effects: [
            // Special handler - removes from sensor detection
        ],
        requirements: #{ min_tier: 4 },
        icon: "cloak",
        tier: 4,
        is_passive: false,
    }
}

fn tractor_beam() {
    #{
        id: "tractor_beam",
        name: "Tractor Beam",
        description: "Lock onto target to prevent escape.",
        cooldown: 45,
        cost: #{ energy: 25 },
        target: "any",
        effects: [
            #{ effect_id: "tractor", params: #{ duration_ticks: 40, pull_strength: 0.5 } },
            #{ effect_id: "slow", params: #{ modifier: -0.6, duration_ticks: 40 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "tractor",
        tier: 2,
        is_passive: false,
    }
}

// =============================================================================
// Support Abilities
// =============================================================================

fn repair_drone() {
    #{
        id: "repair_drone",
        name: "Repair Drone",
        description: "Deploy drone to repair friendly ship.",
        cooldown: 90,
        cost: #{ energy: 30, credits: 200 },
        target: "friendly",
        effects: [
            #{ effect_id: "hull_regen", params: #{ heal_per_tick: 3.0, duration_ticks: 60 } },
        ],
        requirements: #{ min_tier: 3 },
        icon: "repair_drone",
        tier: 3,
        is_passive: false,
    }
}

fn shield_transfer() {
    #{
        id: "shield_transfer",
        name: "Shield Transfer",
        description: "Transfer your shield energy to an ally.",
        cooldown: 60,
        cost: #{ shields: 30 },  // Costs own shields
        target: "friendly",
        effects: [
            #{ effect_id: "shield_restore", params: #{ amount: 30 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "shield_transfer",
        tier: 2,
        is_passive: false,
    }
}

fn deploy_fighters() {
    #{
        id: "deploy_fighters",
        name: "Deploy Fighters",
        description: "Launch fighter squadron from carrier.",
        cooldown: 120,
        cost: #{ energy: 50, ammunition: 30 },
        target: "self",
        effects: [
            // Special handler - spawns fighter NPCs
        ],
        requirements: #{ min_tier: 4, ship_classes: ["carrier"] },
        icon: "fighters",
        tier: 4,
        is_passive: false,
    }
}

// =============================================================================
// Passive Abilities
// =============================================================================

fn veteran_pilot() {
    #{
        id: "veteran_pilot",
        name: "Veteran Pilot",
        description: "Experience grants improved accuracy and critical chance.",
        cooldown: 0,
        cost: #{},
        target: "self",
        effects: [
            #{ effect_id: "accuracy_boost", params: #{ modifier: 0.05 } },
            #{ effect_id: "critical_boost", params: #{ crit_bonus: 0.05 } },
        ],
        requirements: #{ min_fame: 500 },
        icon: "veteran",
        tier: 2,
        is_passive: true,
    }
}

fn hardened_hull() {
    #{
        id: "hardened_hull",
        name: "Hardened Hull",
        description: "Reinforced hull provides passive damage reduction.",
        cooldown: 0,
        cost: #{},
        target: "self",
        effects: [
            #{ effect_id: "defense_boost", params: #{ modifier: 0.1 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "hull_hard",
        tier: 2,
        is_passive: true,
    }
}

fn shield_capacitor() {
    #{
        id: "shield_capacitor",
        name: "Shield Capacitor",
        description: "Improved capacitors increase shield regeneration.",
        cooldown: 0,
        cost: #{},
        target: "self",
        effects: [
            #{ effect_id: "shield_regen", params: #{ heal_per_tick: 0.5 } },
        ],
        requirements: #{ min_tier: 2 },
        icon: "capacitor",
        tier: 2,
        is_passive: true,
    }
}

fn smugglers_hold() {
    #{
        id: "smugglers_hold",
        name: "Smuggler's Hold",
        description: "Hidden compartments reduce contraband detection chance.",
        cooldown: 0,
        cost: #{},
        target: "self",
        effects: [
            // Passive effect - reduces scan detection
        ],
        requirements: #{ min_reputation: -50 },  // Must have shady reputation
        icon: "hidden",
        tier: 2,
        is_passive: true,
    }
}

fn merchant_contacts() {
    #{
        id: "merchant_contacts",
        name: "Merchant Contacts",
        description: "Trade connections provide better prices.",
        cooldown: 0,
        cost: #{},
        target: "self",
        effects: [
            // Passive effect - 5% better buy/sell prices
        ],
        requirements: #{ min_fame: 200 },
        icon: "merchant",
        tier: 1,
        is_passive: true,
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all ability definitions.
/// This function is called by the ArchetypeRegistry to load abilities.
fn all_abilities() {
    [
        // Defensive
        emergency_shields(),
        shield_overcharge(),
        emergency_repair(),
        damage_control(),

        // Offensive
        overdrive(),
        targeting_computer(),
        emp_burst(),
        missile_barrage(),
        focused_strike(),

        // Movement
        afterburner(),
        emergency_jump(),
        evasive_maneuvers(),

        // Utility
        scan_target(),
        distress_beacon(),
        cloak(),
        tractor_beam(),

        // Support
        repair_drone(),
        shield_transfer(),
        deploy_fighters(),

        // Passive
        veteran_pilot(),
        hardened_hull(),
        shield_capacitor(),
        smugglers_hold(),
        merchant_contacts(),
    ]
}
