// =============================================================================
// Ship Class Definitions
// =============================================================================
// All ship classes are defined here. Admins can add new ships or modify
// existing ones without touching Rust code.
//
// Required function: all_ships() - returns array of all ship definitions

// Import base templates (when Rhai import is configured)
// import "definitions/base" as base;

// For now, define inline helpers until imports are set up
fn merge(base, over) {
    let result = #{};
    for key in base.keys() {
        result[key] = base[key];
    }
    for key in over.keys() {
        result[key] = over[key];
    }
    result
}

// =============================================================================
// Player Ship Classes
// =============================================================================

fn patrol_corvette() {
    #{
        id: "patrol_corvette",
        name: "Patrol Corvette",
        description: "Light, fast patrol vessel. Ideal for new pilots.",
        tier: 1,
        attack: 30.0,
        defense: 20.0,
        speed: 80.0,
        shield_capacity: 50.0,
        sensor_range: 100.0,
        cargo_capacity: 50,
        fuel_per_sector: 5.0,
        ammo_per_attack: 2.0,
        weapons: [
            #{ type: "railgun", damage: 20.0, accuracy: 0.7, ammo_cost: 1.0 },
            #{ type: "point_defense", damage: 5.0, accuracy: 0.9, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["weapon", "weapon", "utility"],
        is_player_class: true,
        is_hostile: false,
        behaviors: [],
    }
}

fn frigate() {
    #{
        id: "frigate",
        name: "Frigate",
        description: "Balanced multi-role vessel with good firepower and defense.",
        tier: 2,
        attack: 50.0,
        defense: 40.0,
        speed: 60.0,
        shield_capacity: 80.0,
        sensor_range: 120.0,
        cargo_capacity: 100,
        fuel_per_sector: 8.0,
        ammo_per_attack: 3.0,
        weapons: [
            #{ type: "railgun", damage: 25.0, accuracy: 0.75, ammo_cost: 1.5 },
            #{ type: "missile_launcher", damage: 40.0, accuracy: 0.6, ammo_cost: 3.0 },
            #{ type: "point_defense", damage: 8.0, accuracy: 0.9, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["weapon", "weapon", "weapon", "utility", "utility"],
        is_player_class: true,
        is_hostile: false,
        behaviors: [],
        unlock: #{
            fame: 100,
        },
    }
}

fn destroyer() {
    #{
        id: "destroyer",
        name: "Destroyer",
        description: "Heavy combat vessel designed for sustained engagements.",
        tier: 3,
        attack: 80.0,
        defense: 60.0,
        speed: 50.0,
        shield_capacity: 100.0,
        sensor_range: 150.0,
        cargo_capacity: 150,
        fuel_per_sector: 12.0,
        ammo_per_attack: 5.0,
        weapons: [
            #{ type: "railgun", damage: 35.0, accuracy: 0.8, ammo_cost: 2.0 },
            #{ type: "railgun", damage: 35.0, accuracy: 0.8, ammo_cost: 2.0 },
            #{ type: "missile_launcher", damage: 60.0, accuracy: 0.65, ammo_cost: 4.0 },
            #{ type: "point_defense", damage: 10.0, accuracy: 0.9, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["weapon", "weapon", "weapon", "utility", "utility", "special"],
        is_player_class: true,
        is_hostile: false,
        behaviors: [],
        unlock: #{
            fame: 300,
            reputation: 100,
        },
    }
}

fn cruiser() {
    #{
        id: "cruiser",
        name: "Cruiser",
        description: "Powerful warship with heavy armament and thick armor.",
        tier: 4,
        attack: 100.0,
        defense: 80.0,
        speed: 40.0,
        shield_capacity: 150.0,
        sensor_range: 200.0,
        cargo_capacity: 300,
        fuel_per_sector: 15.0,
        ammo_per_attack: 8.0,
        weapons: [
            #{ type: "laser_battery", damage: 50.0, accuracy: 0.85, ammo_cost: 3.0 },
            #{ type: "railgun", damage: 40.0, accuracy: 0.8, ammo_cost: 2.5 },
            #{ type: "missile_launcher", damage: 80.0, accuracy: 0.7, ammo_cost: 5.0 },
            #{ type: "point_defense", damage: 15.0, accuracy: 0.95, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["weapon", "weapon", "weapon", "weapon", "utility", "utility", "special"],
        is_player_class: true,
        is_hostile: false,
        behaviors: [],
        unlock: #{
            fame: 600,
            reputation: 300,
        },
    }
}

fn carrier() {
    #{
        id: "carrier",
        name: "Carrier",
        description: "Capital ship that deploys fighter squadrons. Slow but devastating.",
        tier: 5,
        attack: 40.0,  // Relies on fighters
        defense: 100.0,
        speed: 30.0,
        shield_capacity: 200.0,
        sensor_range: 300.0,
        cargo_capacity: 500,
        fuel_per_sector: 20.0,
        ammo_per_attack: 10.0,
        weapons: [
            #{ type: "point_defense", damage: 20.0, accuracy: 0.95, ammo_cost: 0.5 },
            #{ type: "point_defense", damage: 20.0, accuracy: 0.95, ammo_cost: 0.5 },
            #{ type: "missile_launcher", damage: 100.0, accuracy: 0.6, ammo_cost: 8.0 },
        ],
        upgrade_slots: ["weapon", "weapon", "hangar", "hangar", "utility", "utility", "special"],
        is_player_class: true,
        is_hostile: false,
        behaviors: [],
        unlock: #{
            fame: 1000,
            reputation: 500,
        },
    }
}

// =============================================================================
// Civilian NPC Ships
// =============================================================================

fn freighter() {
    #{
        id: "freighter",
        name: "Freighter",
        description: "Large cargo vessel. Slow and poorly armed but valuable cargo.",
        tier: 1,
        attack: 5.0,
        defense: 15.0,
        speed: 40.0,
        shield_capacity: 20.0,
        sensor_range: 50.0,
        cargo_capacity: 1000,
        fuel_per_sector: 10.0,
        ammo_per_attack: 1.0,
        weapons: [
            #{ type: "point_defense", damage: 3.0, accuracy: 0.6, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["utility"],
        is_player_class: false,
        is_hostile: false,
        behaviors: ["civilian", "trader"],
    }
}

fn transport() {
    #{
        id: "transport",
        name: "Transport",
        description: "Medium cargo vessel with better defenses than freighters.",
        tier: 1,
        attack: 10.0,
        defense: 20.0,
        speed: 50.0,
        shield_capacity: 30.0,
        sensor_range: 60.0,
        cargo_capacity: 500,
        fuel_per_sector: 8.0,
        ammo_per_attack: 1.0,
        weapons: [
            #{ type: "railgun", damage: 10.0, accuracy: 0.5, ammo_cost: 0.5 },
        ],
        upgrade_slots: ["utility", "utility"],
        is_player_class: false,
        is_hostile: false,
        behaviors: ["civilian", "trader"],
    }
}

fn mining_vessel() {
    #{
        id: "mining_vessel",
        name: "Mining Vessel",
        description: "Specialized ship for asteroid mining operations.",
        tier: 1,
        attack: 5.0,
        defense: 25.0,
        speed: 30.0,
        shield_capacity: 40.0,
        sensor_range: 40.0,
        cargo_capacity: 800,
        fuel_per_sector: 12.0,
        ammo_per_attack: 1.0,
        weapons: [
            #{ type: "mining_laser", damage: 5.0, accuracy: 0.9, ammo_cost: 0.2 },
        ],
        upgrade_slots: ["utility"],
        is_player_class: false,
        is_hostile: false,
        behaviors: ["civilian", "miner"],
    }
}

// =============================================================================
// Pirate Ships
// =============================================================================

fn pirate_raider() {
    #{
        id: "pirate_raider",
        name: "Pirate Raider",
        description: "Fast and aggressive raider. Dangerous in packs.",
        tier: 1,
        attack: 40.0,
        defense: 15.0,
        speed: 90.0,
        shield_capacity: 30.0,
        sensor_range: 80.0,
        cargo_capacity: 100,
        fuel_per_sector: 4.0,
        ammo_per_attack: 2.0,
        weapons: [
            #{ type: "railgun", damage: 25.0, accuracy: 0.65, ammo_cost: 1.0 },
            #{ type: "railgun", damage: 25.0, accuracy: 0.65, ammo_cost: 1.0 },
        ],
        upgrade_slots: ["weapon", "weapon"],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["pirate", "raider", "combat"],
    }
}

fn pirate_frigate() {
    #{
        id: "pirate_frigate",
        name: "Pirate Frigate",
        description: "Heavy pirate vessel. Commands raider packs.",
        tier: 2,
        attack: 60.0,
        defense: 35.0,
        speed: 55.0,
        shield_capacity: 60.0,
        sensor_range: 100.0,
        cargo_capacity: 200,
        fuel_per_sector: 7.0,
        ammo_per_attack: 4.0,
        weapons: [
            #{ type: "railgun", damage: 30.0, accuracy: 0.7, ammo_cost: 1.5 },
            #{ type: "missile_launcher", damage: 50.0, accuracy: 0.55, ammo_cost: 3.0 },
        ],
        upgrade_slots: ["weapon", "weapon", "utility"],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["pirate", "leader", "combat"],
    }
}

fn terrorist_bomber() {
    #{
        id: "terrorist_bomber",
        name: "Terrorist Bomber",
        description: "Suicide attacker. Extremely dangerous but fragile.",
        tier: 1,
        attack: 150.0,  // High damage, suicide attacks
        defense: 10.0,
        speed: 70.0,
        shield_capacity: 10.0,
        sensor_range: 50.0,
        cargo_capacity: 50,  // Bomb payload
        fuel_per_sector: 3.0,
        ammo_per_attack: 50.0,  // One big attack
        weapons: [
            #{ type: "bomb", damage: 150.0, accuracy: 0.9, ammo_cost: 50.0 },
        ],
        upgrade_slots: [],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["terrorist", "suicide", "combat"],
    }
}

// =============================================================================
// Sera (Alien) Ships
// =============================================================================

fn sera_swarm() {
    #{
        id: "sera_swarm",
        name: "Sera Swarm",
        description: "Small alien organisms. Dangerous in large numbers.",
        tier: 1,
        attack: 20.0,  // Many small attacks
        defense: 5.0,
        speed: 100.0,
        shield_capacity: 0.0,  // No shields, organic
        sensor_range: 150.0,
        cargo_capacity: 0,
        fuel_per_sector: 1.0,
        ammo_per_attack: 0.5,
        weapons: [
            #{ type: "bio_spike", damage: 15.0, accuracy: 0.8, ammo_cost: 0.3 },
        ],
        upgrade_slots: [],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["sera", "swarm", "combat"],
    }
}

fn sera_hunter() {
    #{
        id: "sera_hunter",
        name: "Sera Hunter",
        description: "Large alien predator. Extremely dangerous.",
        tier: 3,
        attack: 120.0,
        defense: 70.0,
        speed: 60.0,
        shield_capacity: 50.0,
        sensor_range: 250.0,
        cargo_capacity: 0,
        fuel_per_sector: 5.0,
        ammo_per_attack: 5.0,
        weapons: [
            #{ type: "bio_cannon", damage: 80.0, accuracy: 0.75, ammo_cost: 3.0 },
            #{ type: "bio_spike", damage: 30.0, accuracy: 0.85, ammo_cost: 1.0 },
        ],
        upgrade_slots: [],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["sera", "hunter", "combat"],
    }
}

// =============================================================================
// Drone Ships
// =============================================================================

fn drone_harvester() {
    #{
        id: "drone_harvester",
        name: "Drone Harvester",
        description: "Autonomous mining drone. Defends resource claims aggressively.",
        tier: 2,
        attack: 30.0,
        defense: 50.0,
        speed: 35.0,
        shield_capacity: 30.0,
        sensor_range: 100.0,
        cargo_capacity: 2000,
        fuel_per_sector: 2.0,
        ammo_per_attack: 1.0,
        weapons: [
            #{ type: "mining_laser", damage: 20.0, accuracy: 0.85, ammo_cost: 0.5 },
            #{ type: "point_defense", damage: 10.0, accuracy: 0.8, ammo_cost: 0.3 },
        ],
        upgrade_slots: [],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["drone", "harvester", "combat"],
    }
}

fn drone_swarm() {
    #{
        id: "drone_swarm",
        name: "Drone Swarm",
        description: "Small autonomous combat drones. Attack in coordinated groups.",
        tier: 1,
        attack: 15.0,
        defense: 10.0,
        speed: 80.0,
        shield_capacity: 0.0,
        sensor_range: 80.0,
        cargo_capacity: 0,
        fuel_per_sector: 0.5,
        ammo_per_attack: 0.2,
        weapons: [
            #{ type: "micro_laser", damage: 10.0, accuracy: 0.7, ammo_cost: 0.1 },
        ],
        upgrade_slots: [],
        is_player_class: false,
        is_hostile: true,
        behaviors: ["drone", "swarm", "combat"],
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all ship class definitions.
/// This function is called by the ArchetypeRegistry to load ships.
fn all_ships() {
    [
        // Player ships
        patrol_corvette(),
        frigate(),
        destroyer(),
        cruiser(),
        carrier(),

        // Civilian NPCs
        freighter(),
        transport(),
        mining_vessel(),

        // Pirates
        pirate_raider(),
        pirate_frigate(),
        terrorist_bomber(),

        // Sera aliens
        sera_swarm(),
        sera_hunter(),

        // Drones
        drone_harvester(),
        drone_swarm(),
    ]
}
