// =============================================================================
// Faction Definitions
// =============================================================================
// All factions are defined here. Factions control territory, have relations
// with each other, and modify NPC behavior and combat abilities.
//
// Required function: all_factions() - returns array of all faction definitions
//
// Relations range from -100 (hostile) to +100 (allied)
// Default standing is 0 (neutral)

// =============================================================================
// Major Factions
// =============================================================================

fn continuity_compact() {
    #{
        id: "continuity_compact",
        name: "Continuity Compact",
        tag: "COMPACT",
        description: "The dominant human government, focused on order and stability. Controls the core systems and maintains the largest military fleet.",
        color: "#4488FF",  // Blue
        is_playable: true,
        is_hostile: false,
        is_territorial: true,
        behavior_modifiers: #{
            aggression: 0.3,
            trade_preference: 0.8,
            patrol_range: 3.0,
            flee_threshold: 0.2,
            target_preference: "hostile",
            calls_reinforcements: true,
            surrender_chance: 0.1,
        },
        combat_bonuses: #{
            attack_bonus: 0.0,
            defense_bonus: 0.1,  // +10% defense (well-armored ships)
            speed_bonus: 0.0,
            accuracy_bonus: 0.05,
            shield_bonus: 0.05,
            critical_bonus: 0.0,
        },
        standing_requirements: #{
            station_access: -50,
            mission_access: -20,
            premium_market_access: 30,
            hostile_threshold: -80,
        },
        relations: #{
            argent_flotilla: 60,      // Allied
            independents: 0,          // Neutral
            merchants_guild: 40,      // Friendly
            pirates: -80,             // Hostile
            sera: -100,               // At war
            terrorists: -100,         // At war
            drones: -60,              // Hostile
        },
        icon: "compact_logo",
        home_sectors: ["sol", "alpha_centauri", "tau_ceti"],
        preferred_ships: ["frigate", "destroyer", "cruiser"],
        tier: 5,  // Major power
    }
}

fn argent_flotilla() {
    #{
        id: "argent_flotilla",
        name: "Argent Flotilla",
        tag: "ARGENT",
        description: "Elite naval force allied with the Compact. Known for disciplined combat tactics and advanced weaponry.",
        color: "#SILVER",  // Silver/White
        is_playable: true,
        is_hostile: false,
        is_territorial: true,
        behavior_modifiers: #{
            aggression: 0.5,
            trade_preference: 0.4,
            patrol_range: 4.0,
            flee_threshold: 0.15,
            target_preference: "strongest",
            calls_reinforcements: true,
            surrender_chance: 0.0,  // Never surrender
        },
        combat_bonuses: #{
            attack_bonus: 0.15,  // +15% attack
            defense_bonus: 0.05,
            speed_bonus: 0.0,
            accuracy_bonus: 0.1,  // +10% accuracy
            shield_bonus: 0.0,
            critical_bonus: 0.05,
        },
        standing_requirements: #{
            station_access: -30,
            mission_access: 0,
            premium_market_access: 50,
            hostile_threshold: -70,
        },
        relations: #{
            continuity_compact: 60,
            independents: -10,
            merchants_guild: 20,
            pirates: -90,
            sera: -100,
            terrorists: -100,
            drones: -80,
        },
        icon: "argent_logo",
        home_sectors: ["argent_prime", "nova_station"],
        preferred_ships: ["destroyer", "cruiser", "carrier"],
        tier: 4,
    }
}

fn merchants_guild() {
    #{
        id: "merchants_guild",
        name: "Merchants Guild",
        tag: "GUILD",
        description: "Powerful trade consortium that controls major trade routes. Prefers negotiation over combat.",
        color: "#FFD700",  // Gold
        is_playable: true,
        is_hostile: false,
        is_territorial: false,  // No military territory, just trade posts
        behavior_modifiers: #{
            aggression: 0.1,
            trade_preference: 1.0,  // Maximum trade focus
            patrol_range: 1.0,
            flee_threshold: 0.5,  // Run early
            target_preference: "weakest",
            calls_reinforcements: false,
            surrender_chance: 0.4,
        },
        combat_bonuses: #{
            attack_bonus: 0.0,
            defense_bonus: 0.0,
            speed_bonus: 0.1,  // Fast traders
            accuracy_bonus: 0.0,
            shield_bonus: 0.0,
            critical_bonus: 0.0,
        },
        standing_requirements: #{
            station_access: -60,
            mission_access: -30,
            premium_market_access: 20,
            hostile_threshold: -90,
        },
        relations: #{
            continuity_compact: 40,
            argent_flotilla: 20,
            independents: 30,
            pirates: -40,
            sera: -50,
            terrorists: -60,
            drones: -30,
        },
        icon: "guild_logo",
        home_sectors: ["trade_nexus", "freeport"],
        preferred_ships: ["transport", "freighter"],
        tier: 3,
    }
}

fn independents() {
    #{
        id: "independents",
        name: "Independents",
        tag: "INDIE",
        description: "Loose alliance of freelancers, small traders, and colonists. No central authority.",
        color: "#88CC88",  // Green
        is_playable: true,
        is_hostile: false,
        is_territorial: false,
        behavior_modifiers: #{
            aggression: 0.3,
            trade_preference: 0.6,
            patrol_range: 2.0,
            flee_threshold: 0.4,
            target_preference: (),  // No preference
            calls_reinforcements: false,
            surrender_chance: 0.3,
        },
        combat_bonuses: #{
            attack_bonus: 0.0,
            defense_bonus: 0.0,
            speed_bonus: 0.05,
            accuracy_bonus: 0.0,
            shield_bonus: 0.0,
            critical_bonus: 0.0,
        },
        standing_requirements: #{
            station_access: -70,
            mission_access: -40,
            premium_market_access: 40,
            hostile_threshold: -100,
        },
        relations: #{
            continuity_compact: 0,
            argent_flotilla: -10,
            merchants_guild: 30,
            pirates: -30,
            sera: -70,
            terrorists: -50,
            drones: -40,
        },
        icon: "indie_logo",
        home_sectors: ["frontier_1", "frontier_2", "outer_rim"],
        preferred_ships: ["patrol_corvette", "transport"],
        tier: 2,
    }
}

// =============================================================================
// Hostile Factions
// =============================================================================

fn pirates() {
    #{
        id: "pirates",
        name: "Pirates",
        tag: "PIRATE",
        description: "Criminal raiders who prey on trade routes. Operate from hidden bases in lawless space.",
        color: "#CC4444",  // Red
        is_playable: false,
        is_hostile: true,
        is_territorial: false,  // Mobile, no fixed territory
        behavior_modifiers: #{
            aggression: 0.9,
            trade_preference: 0.0,
            patrol_range: 5.0,  // Wide hunting range
            flee_threshold: 0.35,
            target_preference: "cargo",  // Target cargo ships
            calls_reinforcements: true,
            surrender_chance: 0.2,
        },
        combat_bonuses: #{
            attack_bonus: 0.05,
            defense_bonus: -0.1,  // Weaker armor
            speed_bonus: 0.15,  // Fast raiders
            accuracy_bonus: 0.0,
            shield_bonus: -0.1,
            critical_bonus: 0.1,  // High crit for ambush attacks
        },
        standing_requirements: #{
            station_access: -100,  // Hidden bases only
            mission_access: -80,
            premium_market_access: 0,
            hostile_threshold: -50,
        },
        relations: #{
            continuity_compact: -80,
            argent_flotilla: -90,
            merchants_guild: -40,
            independents: -30,
            sera: -100,
            terrorists: 20,  // Sometimes work together
            drones: -50,
        },
        icon: "pirate_logo",
        home_sectors: ["pirate_haven", "dead_space"],
        preferred_ships: ["pirate_raider", "pirate_frigate"],
        tier: 2,
    }
}

fn terrorists() {
    #{
        id: "terrorists",
        name: "Separatist Terrorists",
        tag: "TERROR",
        description: "Extremist separatists who use suicide tactics. Extremely dangerous.",
        color: "#880000",  // Dark red
        is_playable: false,
        is_hostile: true,
        is_territorial: false,
        behavior_modifiers: #{
            aggression: 1.0,  // Maximum aggression
            trade_preference: 0.0,
            patrol_range: 3.0,
            flee_threshold: 0.0,  // Never flee
            target_preference: "player",  // Target players specifically
            calls_reinforcements: false,
            surrender_chance: 0.0,
        },
        combat_bonuses: #{
            attack_bonus: 0.3,  // Suicide attacks are devastating
            defense_bonus: -0.3,
            speed_bonus: 0.1,
            accuracy_bonus: 0.0,
            shield_bonus: -0.5,
            critical_bonus: 0.2,
        },
        standing_requirements: #{
            station_access: -100,
            mission_access: -100,
            premium_market_access: -100,
            hostile_threshold: 0,  // Always hostile
        },
        relations: #{
            continuity_compact: -100,
            argent_flotilla: -100,
            merchants_guild: -60,
            independents: -50,
            pirates: 20,
            sera: -100,
            drones: -50,
        },
        icon: "terror_logo",
        home_sectors: [],  // Hidden
        preferred_ships: ["terrorist_bomber"],
        tier: 1,
    }
}

fn sera() {
    #{
        id: "sera",
        name: "Sera",
        tag: "SERA",
        description: "Alien organisms of unknown origin. Hostile to all human life. Communication impossible.",
        color: "#AA00AA",  // Purple
        is_playable: false,
        is_hostile: true,
        is_territorial: true,
        behavior_modifiers: #{
            aggression: 0.95,
            trade_preference: 0.0,
            patrol_range: 6.0,
            flee_threshold: 0.1,
            target_preference: (),  // Attack everything
            calls_reinforcements: true,  // Swarm behavior
            surrender_chance: 0.0,
        },
        combat_bonuses: #{
            attack_bonus: 0.1,
            defense_bonus: 0.0,
            speed_bonus: 0.2,
            accuracy_bonus: 0.1,
            shield_bonus: -1.0,  // No shields, organic
            critical_bonus: 0.15,
        },
        standing_requirements: #{
            station_access: -100,
            mission_access: -100,
            premium_market_access: -100,
            hostile_threshold: 100,  // Always hostile, no diplomacy
        },
        relations: #{
            continuity_compact: -100,
            argent_flotilla: -100,
            merchants_guild: -100,
            independents: -100,
            pirates: -100,
            terrorists: -100,
            drones: -100,
        },
        icon: "sera_logo",
        home_sectors: ["sera_hive", "infestation_zone"],
        preferred_ships: ["sera_swarm", "sera_hunter"],
        tier: 4,
    }
}

fn drones() {
    #{
        id: "drones",
        name: "Rogue Drones",
        tag: "DRONE",
        description: "Autonomous mining and combat drones that went rogue. Defend resource claims aggressively.",
        color: "#666666",  // Gray
        is_playable: false,
        is_hostile: true,
        is_territorial: true,
        behavior_modifiers: #{
            aggression: 0.6,
            trade_preference: 0.0,
            patrol_range: 2.0,
            flee_threshold: 0.0,  // Machines don't flee
            target_preference: "nearest",
            calls_reinforcements: true,
            surrender_chance: 0.0,
        },
        combat_bonuses: #{
            attack_bonus: 0.0,
            defense_bonus: 0.2,  // Tough metal hulls
            speed_bonus: -0.1,
            accuracy_bonus: 0.15,  // Computer targeting
            shield_bonus: 0.0,
            critical_bonus: 0.0,  // No lucky shots
        },
        standing_requirements: #{
            station_access: -100,
            mission_access: -100,
            premium_market_access: -100,
            hostile_threshold: -20,  // Become hostile quickly
        },
        relations: #{
            continuity_compact: -60,
            argent_flotilla: -80,
            merchants_guild: -30,
            independents: -40,
            pirates: -50,
            terrorists: -50,
            sera: -100,
        },
        icon: "drone_logo",
        home_sectors: ["abandoned_mines", "drone_fields"],
        preferred_ships: ["drone_harvester", "drone_swarm"],
        tier: 2,
    }
}

// =============================================================================
// Special Factions
// =============================================================================

fn bounty_hunters() {
    #{
        id: "bounty_hunters",
        name: "Bounty Hunters Guild",
        tag: "HUNTER",
        description: "Licensed hunters who track down criminals and collect bounties. Respected by law enforcement.",
        color: "#FF8800",  // Orange
        is_playable: true,
        is_hostile: false,
        is_territorial: false,
        behavior_modifiers: #{
            aggression: 0.7,
            trade_preference: 0.2,
            patrol_range: 8.0,  // Wide hunting range
            flee_threshold: 0.25,
            target_preference: "bounty",  // Target bounties
            calls_reinforcements: false,  // Solo operators
            surrender_chance: 0.0,
        },
        combat_bonuses: #{
            attack_bonus: 0.1,
            defense_bonus: 0.05,
            speed_bonus: 0.1,
            accuracy_bonus: 0.1,
            shield_bonus: 0.0,
            critical_bonus: 0.1,
        },
        standing_requirements: #{
            station_access: -40,
            mission_access: 0,
            premium_market_access: 30,
            hostile_threshold: -60,
        },
        relations: #{
            continuity_compact: 50,
            argent_flotilla: 40,
            merchants_guild: 20,
            independents: 0,
            pirates: -70,
            terrorists: -90,
            sera: -80,
            drones: -40,
        },
        icon: "hunter_logo",
        home_sectors: [],  // Nomadic
        preferred_ships: ["patrol_corvette", "frigate"],
        tier: 2,
    }
}

fn research_consortium() {
    #{
        id: "research_consortium",
        name: "Research Consortium",
        tag: "RESEARCH",
        description: "Scientific organization studying alien technology and phenomena. Neutral toward most factions.",
        color: "#00CCCC",  // Cyan
        is_playable: false,
        is_hostile: false,
        is_territorial: false,
        behavior_modifiers: #{
            aggression: 0.1,
            trade_preference: 0.5,
            patrol_range: 1.0,
            flee_threshold: 0.6,
            target_preference: (),
            calls_reinforcements: false,
            surrender_chance: 0.5,
        },
        combat_bonuses: #{
            attack_bonus: -0.2,  // Not combat focused
            defense_bonus: 0.1,
            speed_bonus: 0.0,
            accuracy_bonus: 0.0,
            shield_bonus: 0.15,  // Advanced shields
            critical_bonus: 0.0,
        },
        standing_requirements: #{
            station_access: -30,
            mission_access: 10,
            premium_market_access: 50,
            hostile_threshold: -80,
        },
        relations: #{
            continuity_compact: 30,
            argent_flotilla: 10,
            merchants_guild: 40,
            independents: 20,
            pirates: -50,
            terrorists: -70,
            sera: -60,  // Study them from a distance
            drones: -20,
        },
        icon: "research_logo",
        home_sectors: ["research_station_alpha"],
        preferred_ships: ["transport"],
        tier: 2,
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all faction definitions.
/// This function is called by the ArchetypeRegistry to load factions.
fn all_factions() {
    [
        // Major factions
        continuity_compact(),
        argent_flotilla(),
        merchants_guild(),
        independents(),

        // Hostile factions
        pirates(),
        terrorists(),
        sera(),
        drones(),

        // Special factions
        bounty_hunters(),
        research_consortium(),
    ]
}
