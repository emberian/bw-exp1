// =============================================================================
// Weapon Definitions
// =============================================================================
// All weapon types are defined here. Admins can add new weapons or modify
// existing ones without touching Rust code.
//
// Required function: all_weapons() - returns array of all weapon definitions

// =============================================================================
// Kinetic Weapons
// =============================================================================

fn railgun() {
    #{
        id: "railgun",
        name: "Railgun",
        description: "High velocity kinetic weapon. Accurate and reliable.",
        damage: 25.0,
        accuracy: 0.75,
        ammo_cost: 1.5,
        range_modifier: 1.0,
        category: "kinetic",
        tier: 1,
        cost: 5000,
        effects: [],
    }
}

fn heavy_railgun() {
    #{
        id: "heavy_railgun",
        name: "Heavy Railgun",
        description: "Large caliber railgun with devastating impact.",
        damage: 45.0,
        accuracy: 0.7,
        ammo_cost: 3.0,
        range_modifier: 1.2,
        category: "kinetic",
        tier: 3,
        cost: 25000,
        effects: ["armor_piercing"],
    }
}

fn autocannon() {
    #{
        id: "autocannon",
        name: "Autocannon",
        description: "Rapid-fire kinetic weapon. High volume of fire.",
        damage: 12.0,
        accuracy: 0.65,
        ammo_cost: 0.5,
        range_modifier: 0.8,
        category: "kinetic",
        tier: 1,
        cost: 3000,
        effects: ["rapid_fire"],
    }
}

// =============================================================================
// Energy Weapons
// =============================================================================

fn laser_battery() {
    #{
        id: "laser_battery",
        name: "Laser Battery",
        description: "Focused energy beam. Consistent damage, effective against shields.",
        damage: 30.0,
        accuracy: 0.85,
        ammo_cost: 2.0,
        range_modifier: 0.9,
        category: "energy",
        tier: 2,
        cost: 15000,
        effects: ["shield_piercing"],
    }
}

fn plasma_cannon() {
    #{
        id: "plasma_cannon",
        name: "Plasma Cannon",
        description: "Superheated plasma projectile. High damage but lower accuracy.",
        damage: 55.0,
        accuracy: 0.6,
        ammo_cost: 4.0,
        range_modifier: 0.7,
        category: "energy",
        tier: 3,
        cost: 35000,
        effects: ["shield_piercing", "burn"],
    }
}

fn pulse_laser() {
    #{
        id: "pulse_laser",
        name: "Pulse Laser",
        description: "Rapid pulse energy weapon. Good for shield stripping.",
        damage: 15.0,
        accuracy: 0.8,
        ammo_cost: 0.8,
        range_modifier: 0.85,
        category: "energy",
        tier: 1,
        cost: 8000,
        effects: ["shield_piercing"],
    }
}

// =============================================================================
// Missile Weapons
// =============================================================================

fn missile_launcher() {
    #{
        id: "missile_launcher",
        name: "Missile Launcher",
        description: "Guided explosive ordnance. High damage, can be shot down.",
        damage: 50.0,
        accuracy: 0.6,
        ammo_cost: 4.0,
        range_modifier: 1.5,
        category: "missile",
        tier: 2,
        cost: 12000,
        effects: ["tracking", "explosive"],
    }
}

fn torpedo_launcher() {
    #{
        id: "torpedo_launcher",
        name: "Torpedo Launcher",
        description: "Heavy anti-ship torpedo. Devastating against large targets.",
        damage: 120.0,
        accuracy: 0.5,
        ammo_cost: 10.0,
        range_modifier: 2.0,
        category: "missile",
        tier: 4,
        cost: 50000,
        effects: ["tracking", "explosive", "armor_piercing"],
    }
}

fn swarm_missiles() {
    #{
        id: "swarm_missiles",
        name: "Swarm Missiles",
        description: "Multiple small missiles. Hard to shoot down.",
        damage: 8.0,
        accuracy: 0.75,
        ammo_cost: 1.0,
        range_modifier: 1.2,
        category: "missile",
        tier: 2,
        cost: 18000,
        effects: ["tracking", "swarm"],
    }
}

// =============================================================================
// Defense Systems
// =============================================================================

fn point_defense() {
    #{
        id: "point_defense",
        name: "Point Defense",
        description: "Anti-missile and anti-fighter system.",
        damage: 8.0,
        accuracy: 0.9,
        ammo_cost: 0.3,
        range_modifier: 0.3,
        category: "defense",
        tier: 1,
        cost: 6000,
        effects: ["anti_missile", "rapid_fire"],
    }
}

fn flak_battery() {
    #{
        id: "flak_battery",
        name: "Flak Battery",
        description: "Area denial weapon. Effective against swarms.",
        damage: 15.0,
        accuracy: 0.7,
        ammo_cost: 1.0,
        range_modifier: 0.5,
        category: "defense",
        tier: 2,
        cost: 10000,
        effects: ["anti_missile", "area_damage"],
    }
}

// =============================================================================
// Special Weapons
// =============================================================================

fn mining_laser() {
    #{
        id: "mining_laser",
        name: "Mining Laser",
        description: "Industrial laser for asteroid mining. Poor combat effectiveness.",
        damage: 5.0,
        accuracy: 0.95,
        ammo_cost: 0.2,
        range_modifier: 0.3,
        category: "utility",
        tier: 1,
        cost: 2000,
        effects: ["mining"],
    }
}

fn emp_cannon() {
    #{
        id: "emp_cannon",
        name: "EMP Cannon",
        description: "Electromagnetic pulse weapon. Disables ship systems.",
        damage: 10.0,
        accuracy: 0.7,
        ammo_cost: 5.0,
        range_modifier: 0.8,
        category: "special",
        tier: 4,
        cost: 75000,
        effects: ["emp", "disable"],
    }
}

fn tractor_beam() {
    #{
        id: "tractor_beam",
        name: "Tractor Beam",
        description: "Gravitational manipulation beam. Slows targets.",
        damage: 0.0,
        accuracy: 0.85,
        ammo_cost: 3.0,
        range_modifier: 0.6,
        category: "utility",
        tier: 3,
        cost: 30000,
        effects: ["slow", "tractor"],
    }
}

// =============================================================================
// Alien/Exotic Weapons
// =============================================================================

fn bio_spike() {
    #{
        id: "bio_spike",
        name: "Bio Spike",
        description: "Organic projectile used by Sera organisms.",
        damage: 15.0,
        accuracy: 0.8,
        ammo_cost: 0.3,
        range_modifier: 0.7,
        category: "organic",
        tier: 1,
        cost: 0,  // Not purchasable
        effects: ["organic", "regenerating"],
    }
}

fn bio_cannon() {
    #{
        id: "bio_cannon",
        name: "Bio Cannon",
        description: "Large organic weapon used by Sera hunters.",
        damage: 80.0,
        accuracy: 0.75,
        ammo_cost: 3.0,
        range_modifier: 1.0,
        category: "organic",
        tier: 3,
        cost: 0,  // Not purchasable
        effects: ["organic", "acid"],
    }
}

fn micro_laser() {
    #{
        id: "micro_laser",
        name: "Micro Laser",
        description: "Compact laser used by drone swarms.",
        damage: 10.0,
        accuracy: 0.7,
        ammo_cost: 0.1,
        range_modifier: 0.5,
        category: "energy",
        tier: 1,
        cost: 0,  // Not purchasable
        effects: [],
    }
}

fn bomb() {
    #{
        id: "bomb",
        name: "Bomb",
        description: "Single-use explosive payload. Used by suicide attackers.",
        damage: 150.0,
        accuracy: 0.9,
        ammo_cost: 50.0,
        range_modifier: 0.2,
        category: "special",
        tier: 1,
        cost: 0,  // Not purchasable
        effects: ["explosive", "single_use"],
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all weapon definitions.
/// This function is called by the ArchetypeRegistry to load weapons.
fn all_weapons() {
    [
        // Kinetic
        railgun(),
        heavy_railgun(),
        autocannon(),

        // Energy
        laser_battery(),
        plasma_cannon(),
        pulse_laser(),

        // Missiles
        missile_launcher(),
        torpedo_launcher(),
        swarm_missiles(),

        // Defense
        point_defense(),
        flak_battery(),

        // Special
        mining_laser(),
        emp_cannon(),
        tractor_beam(),

        // Alien/Exotic
        bio_spike(),
        bio_cannon(),
        micro_laser(),
        bomb(),
    ]
}
