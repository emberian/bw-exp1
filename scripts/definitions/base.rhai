// =============================================================================
// Base Definitions - Shared constants and helper functions
// =============================================================================
// This file provides reusable building blocks for other definition files.
// Import with: import "definitions/base" as base;

// =============================================================================
// Ship Class Base Templates
// =============================================================================

/// Base stats for corvette-class ships
fn corvette_base() {
    #{
        tier: 1,
        speed: 80.0,
        shield_capacity: 50.0,
        sensor_range: 100.0,
        cargo_capacity: 50,
        fuel_per_sector: 5.0,
        ammo_per_attack: 2.0,
        upgrade_slots: ["weapon", "weapon", "utility"],
    }
}

/// Base stats for frigate-class ships
fn frigate_base() {
    #{
        tier: 2,
        speed: 60.0,
        shield_capacity: 80.0,
        sensor_range: 120.0,
        cargo_capacity: 100,
        fuel_per_sector: 8.0,
        ammo_per_attack: 3.0,
        upgrade_slots: ["weapon", "weapon", "weapon", "utility", "utility"],
    }
}

/// Base stats for destroyer-class ships
fn destroyer_base() {
    #{
        tier: 3,
        speed: 50.0,
        shield_capacity: 100.0,
        sensor_range: 150.0,
        cargo_capacity: 150,
        fuel_per_sector: 12.0,
        ammo_per_attack: 5.0,
        upgrade_slots: ["weapon", "weapon", "weapon", "utility", "utility", "special"],
    }
}

/// Base stats for cruiser-class ships
fn cruiser_base() {
    #{
        tier: 4,
        speed: 40.0,
        shield_capacity: 150.0,
        sensor_range: 200.0,
        cargo_capacity: 300,
        fuel_per_sector: 15.0,
        ammo_per_attack: 8.0,
        upgrade_slots: ["weapon", "weapon", "weapon", "weapon", "utility", "utility", "special"],
    }
}

/// Base stats for carrier-class ships
fn carrier_base() {
    #{
        tier: 5,
        speed: 30.0,
        shield_capacity: 200.0,
        sensor_range: 300.0,
        cargo_capacity: 500,
        fuel_per_sector: 20.0,
        ammo_per_attack: 10.0,
        upgrade_slots: ["weapon", "weapon", "hangar", "hangar", "utility", "utility", "special"],
    }
}

/// Base stats for civilian freighter ships
fn civilian_freighter_base() {
    #{
        tier: 1,
        speed: 40.0,
        shield_capacity: 20.0,
        sensor_range: 50.0,
        cargo_capacity: 1000,
        fuel_per_sector: 10.0,
        ammo_per_attack: 1.0,
        upgrade_slots: ["utility"],
        is_hostile: false,
    }
}

/// Base stats for pirate raider ships
fn pirate_raider_base() {
    #{
        tier: 1,
        speed: 90.0,
        shield_capacity: 30.0,
        sensor_range: 80.0,
        cargo_capacity: 100,
        fuel_per_sector: 4.0,
        ammo_per_attack: 2.0,
        upgrade_slots: ["weapon", "weapon"],
        is_hostile: true,
        behaviors: ["pirate", "combat"],
    }
}

/// Base stats for alien Sera ships
fn sera_base() {
    #{
        tier: 3,
        shield_capacity: 0.0,  // Organic, no shields
        sensor_range: 150.0,
        cargo_capacity: 0,
        fuel_per_sector: 1.0,
        ammo_per_attack: 0.5,
        is_hostile: true,
        behaviors: ["sera", "swarm", "combat"],
    }
}

/// Base stats for drone ships
fn drone_base() {
    #{
        tier: 2,
        shield_capacity: 30.0,
        sensor_range: 100.0,
        fuel_per_sector: 2.0,
        ammo_per_attack: 1.0,
        is_hostile: true,
        behaviors: ["drone", "harvest", "combat"],
    }
}

// =============================================================================
// Weapon Base Templates
// =============================================================================

/// Base stats for kinetic weapons
fn kinetic_weapon_base() {
    #{
        category: "kinetic",
        range_modifier: 1.0,
        effects: [],
    }
}

/// Base stats for energy weapons
fn energy_weapon_base() {
    #{
        category: "energy",
        range_modifier: 0.8,
        effects: ["shield_piercing"],
    }
}

/// Base stats for missile weapons
fn missile_weapon_base() {
    #{
        category: "missile",
        range_modifier: 1.5,
        effects: ["tracking"],
    }
}

/// Base stats for point defense systems
fn point_defense_base() {
    #{
        category: "defense",
        range_modifier: 0.3,
        effects: ["anti_missile"],
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Merge two maps, with the second overriding the first
fn merge(base, override) {
    let result = #{};
    for key in base.keys() {
        result[key] = base[key];
    }
    for key in override.keys() {
        result[key] = override[key];
    }
    result
}

/// Scale stats by a multiplier
fn scale_stats(stats, mult) {
    #{
        attack: stats.attack * mult,
        defense: stats.defense * mult,
        speed: stats.speed,
        shield_capacity: stats.shield_capacity * mult,
        sensor_range: stats.sensor_range,
        cargo_capacity: stats.cargo_capacity,
        fuel_per_sector: stats.fuel_per_sector,
        ammo_per_attack: stats.ammo_per_attack,
    }
}

/// Create a standard railgun weapon
fn railgun(damage, accuracy) {
    merge(kinetic_weapon_base(), #{
        type: "railgun",
        damage: damage,
        accuracy: accuracy,
        ammo_cost: damage / 20.0,
    })
}

/// Create a standard missile launcher
fn missile(damage, accuracy) {
    merge(missile_weapon_base(), #{
        type: "missile_launcher",
        damage: damage,
        accuracy: accuracy,
        ammo_cost: damage / 15.0,
    })
}

/// Create a standard laser battery
fn laser(damage, accuracy) {
    merge(energy_weapon_base(), #{
        type: "laser_battery",
        damage: damage,
        accuracy: accuracy,
        ammo_cost: damage / 25.0,
    })
}

/// Create a point defense system
fn point_defense(damage, accuracy) {
    merge(point_defense_base(), #{
        type: "point_defense",
        damage: damage,
        accuracy: accuracy,
        ammo_cost: 0.5,
    })
}
