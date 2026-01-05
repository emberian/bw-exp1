// =============================================================================
// Cargo Definitions
// =============================================================================
// All tradeable cargo types are defined here. Cargo can be bought, sold,
// transported, and sometimes smuggled across the galaxy.
//
// Required function: all_cargo() - returns array of all cargo definitions
//
// Categories:
// - consumable: Used by ships (fuel, ammo)
// - industrial: Raw materials and components
// - high_value: Rare and expensive goods
// - contraband: Illegal goods with high risk/reward
// - military: Weapons and military equipment
// - data: Information and software

// =============================================================================
// Consumables
// =============================================================================

fn fuel() {
    #{
        id: "fuel",
        name: "Fuel",
        description: "Standard ship fuel for FTL jumps.",
        base_price: 50,
        weight: 1,
        legal: true,
        volatility: 0.1,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "fuel",
        tier: 1,
        abundance: 0.9,  // Very common
        demand_modifiers: #{
            outpost: 1.2,    // Higher demand at outposts
            mining: 0.9,     // Lower at mining stations
            military: 1.1,
        },
    }
}

fn ammunition() {
    #{
        id: "ammunition",
        name: "Ammunition",
        description: "Standard kinetic weapon ammunition.",
        base_price: 80,
        weight: 2,
        legal: true,
        volatility: 0.15,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "ammo",
        tier: 1,
        abundance: 0.7,
        demand_modifiers: #{
            military: 1.5,
            pirate: 1.3,
            civilian: 0.7,
        },
    }
}

fn energy_cells() {
    #{
        id: "energy_cells",
        name: "Energy Cells",
        description: "Rechargeable energy storage for ship systems.",
        base_price: 120,
        weight: 1,
        legal: true,
        volatility: 0.12,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "energy",
        tier: 1,
        abundance: 0.6,
        demand_modifiers: #{
            research: 1.3,
            military: 1.2,
        },
    }
}

fn medical_supplies() {
    #{
        id: "medical_supplies",
        name: "Medical Supplies",
        description: "Medicine and medical equipment.",
        base_price: 200,
        weight: 1,
        legal: true,
        volatility: 0.2,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "medical",
        tier: 2,
        abundance: 0.5,
        demand_modifiers: #{
            civilian: 1.4,
            outpost: 1.6,
            military: 1.2,
        },
    }
}

// =============================================================================
// Industrial Goods
// =============================================================================

fn ore() {
    #{
        id: "ore",
        name: "Ore",
        description: "Unprocessed mineral ore from asteroid mining.",
        base_price: 80,
        weight: 3,
        legal: true,
        volatility: 0.15,
        contraband_penalty: 0,
        category: "industrial",
        special_effects: [],
        icon: "ore",
        tier: 1,
        abundance: 0.8,
        demand_modifiers: #{
            industrial: 1.5,
            mining: 0.6,  // Surplus at mining stations
        },
    }
}

fn refined_metals() {
    #{
        id: "refined_metals",
        name: "Refined Metals",
        description: "Processed metals for manufacturing.",
        base_price: 150,
        weight: 2,
        legal: true,
        volatility: 0.12,
        contraband_penalty: 0,
        category: "industrial",
        special_effects: [],
        icon: "metal",
        tier: 2,
        abundance: 0.6,
        demand_modifiers: #{
            shipyard: 1.8,
            industrial: 1.4,
        },
    }
}

fn electronics() {
    #{
        id: "electronics",
        name: "Electronics",
        description: "Electronic components and circuits.",
        base_price: 250,
        weight: 1,
        legal: true,
        volatility: 0.2,
        contraband_penalty: 0,
        category: "industrial",
        special_effects: [],
        icon: "electronics",
        tier: 2,
        abundance: 0.5,
        demand_modifiers: #{
            research: 1.5,
            shipyard: 1.3,
            outpost: 1.4,
        },
    }
}

fn construction_materials() {
    #{
        id: "construction_materials",
        name: "Construction Materials",
        description: "Prefabricated building materials.",
        base_price: 100,
        weight: 4,
        legal: true,
        volatility: 0.1,
        contraband_penalty: 0,
        category: "industrial",
        special_effects: [],
        icon: "construction",
        tier: 1,
        abundance: 0.7,
        demand_modifiers: #{
            outpost: 1.6,
            shipyard: 1.3,
        },
    }
}

fn machinery() {
    #{
        id: "machinery",
        name: "Machinery",
        description: "Industrial machinery and equipment.",
        base_price: 400,
        weight: 5,
        legal: true,
        volatility: 0.15,
        contraband_penalty: 0,
        category: "industrial",
        special_effects: [],
        icon: "machinery",
        tier: 3,
        abundance: 0.4,
        demand_modifiers: #{
            industrial: 1.4,
            mining: 1.5,
        },
    }
}

// =============================================================================
// High Value Goods
// =============================================================================

fn rare_earths() {
    #{
        id: "rare_earths",
        name: "Rare Earths",
        description: "Rare elements essential for advanced technology.",
        base_price: 500,
        weight: 1,
        legal: true,
        volatility: 0.3,
        contraband_penalty: 0,
        category: "high_value",
        special_effects: [],
        icon: "rare",
        tier: 3,
        abundance: 0.2,
        demand_modifiers: #{
            research: 2.0,
            military: 1.5,
            shipyard: 1.6,
        },
    }
}

fn luxury_goods() {
    #{
        id: "luxury_goods",
        name: "Luxury Goods",
        description: "High-end consumer goods and art.",
        base_price: 600,
        weight: 1,
        legal: true,
        volatility: 0.35,
        contraband_penalty: 0,
        category: "high_value",
        special_effects: [],
        icon: "luxury",
        tier: 3,
        abundance: 0.3,
        demand_modifiers: #{
            hub: 1.8,
            outpost: 0.6,
        },
    }
}

fn alien_artifacts() {
    #{
        id: "alien_artifacts",
        name: "Alien Artifacts",
        description: "Mysterious objects of unknown origin.",
        base_price: 1000,
        weight: 1,
        legal: true,
        volatility: 0.5,
        contraband_penalty: 0,
        category: "high_value",
        special_effects: ["mysterious"],
        icon: "artifact",
        tier: 4,
        abundance: 0.1,
        demand_modifiers: #{
            research: 3.0,
            hub: 1.5,
        },
    }
}

fn quantum_cores() {
    #{
        id: "quantum_cores",
        name: "Quantum Cores",
        description: "Quantum computing cores for advanced systems.",
        base_price: 2000,
        weight: 1,
        legal: true,
        volatility: 0.25,
        contraband_penalty: 0,
        category: "high_value",
        special_effects: [],
        icon: "quantum",
        tier: 5,
        abundance: 0.05,
        demand_modifiers: #{
            research: 2.5,
            military: 2.0,
        },
    }
}

// =============================================================================
// Food & Agriculture
// =============================================================================

fn food() {
    #{
        id: "food",
        name: "Food",
        description: "Basic food supplies for stations and colonies.",
        base_price: 60,
        weight: 1,
        legal: true,
        volatility: 0.1,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "food",
        tier: 1,
        abundance: 0.7,
        demand_modifiers: #{
            outpost: 1.8,
            mining: 1.5,
            agricultural: 0.5,
        },
    }
}

fn water() {
    #{
        id: "water",
        name: "Water",
        description: "Purified water for consumption and industry.",
        base_price: 40,
        weight: 2,
        legal: true,
        volatility: 0.05,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: [],
        icon: "water",
        tier: 1,
        abundance: 0.8,
        demand_modifiers: #{
            outpost: 1.5,
            desert: 2.5,
        },
    }
}

fn livestock() {
    #{
        id: "livestock",
        name: "Livestock",
        description: "Live animals for farming or consumption.",
        base_price: 150,
        weight: 3,
        legal: true,
        volatility: 0.2,
        contraband_penalty: 0,
        category: "consumable",
        special_effects: ["perishable"],
        icon: "livestock",
        tier: 2,
        abundance: 0.4,
        demand_modifiers: #{
            agricultural: 0.7,
            outpost: 1.6,
        },
    }
}

// =============================================================================
// Contraband
// =============================================================================

fn stolen_goods() {
    #{
        id: "stolen_goods",
        name: "Stolen Goods",
        description: "Goods of dubious origin. Highly illegal.",
        base_price: 200,
        weight: 1,
        legal: false,
        volatility: 0.5,
        contraband_penalty: 50,
        category: "contraband",
        special_effects: [],
        icon: "stolen",
        tier: 1,
        abundance: 0.3,
        demand_modifiers: #{
            pirate: 0.8,
            black_market: 1.5,
        },
    }
}

fn narcotics() {
    #{
        id: "narcotics",
        name: "Narcotics",
        description: "Illegal drugs. Severe penalties if caught.",
        base_price: 400,
        weight: 1,
        legal: false,
        volatility: 0.6,
        contraband_penalty: 100,
        category: "contraband",
        special_effects: [],
        icon: "drugs",
        tier: 2,
        abundance: 0.2,
        demand_modifiers: #{
            black_market: 2.0,
            hub: 1.3,
        },
    }
}

fn counterfeit_credits() {
    #{
        id: "counterfeit_credits",
        name: "Counterfeit Credits",
        description: "Fake currency. Extremely risky to trade.",
        base_price: 300,
        weight: 1,
        legal: false,
        volatility: 0.8,
        contraband_penalty: 150,
        category: "contraband",
        special_effects: [],
        icon: "counterfeit",
        tier: 3,
        abundance: 0.1,
        demand_modifiers: #{
            black_market: 1.8,
        },
    }
}

fn slaves() {
    #{
        id: "slaves",
        name: "Slaves",
        description: "Human trafficking. Maximum penalties. Reputation destroyer.",
        base_price: 500,
        weight: 2,
        legal: false,
        volatility: 0.4,
        contraband_penalty: 300,
        category: "contraband",
        special_effects: ["reputation_penalty"],
        icon: "slaves",
        tier: 3,
        abundance: 0.05,
        demand_modifiers: #{
            pirate: 1.5,
        },
    }
}

fn biological_weapons() {
    #{
        id: "biological_weapons",
        name: "Biological Weapons",
        description: "Banned weapons of mass destruction.",
        base_price: 2000,
        weight: 1,
        legal: false,
        volatility: 0.3,
        contraband_penalty: 500,
        category: "contraband",
        special_effects: [],
        icon: "biohazard",
        tier: 5,
        abundance: 0.02,
        demand_modifiers: #{
            terrorist: 3.0,
        },
    }
}

// =============================================================================
// Military Goods
// =============================================================================

fn weapons_components() {
    #{
        id: "weapons_components",
        name: "Weapons Components",
        description: "Parts for constructing and repairing weapons.",
        base_price: 300,
        weight: 2,
        legal: true,  // Legal but regulated
        volatility: 0.2,
        contraband_penalty: 0,
        category: "military",
        special_effects: [],
        icon: "weapons",
        tier: 2,
        abundance: 0.4,
        demand_modifiers: #{
            military: 1.6,
            shipyard: 1.4,
            pirate: 1.3,
        },
    }
}

fn shield_generators() {
    #{
        id: "shield_generators",
        name: "Shield Generators",
        description: "Portable shield generation equipment.",
        base_price: 450,
        weight: 3,
        legal: true,
        volatility: 0.18,
        contraband_penalty: 0,
        category: "military",
        special_effects: [],
        icon: "shield_gen",
        tier: 3,
        abundance: 0.35,
        demand_modifiers: #{
            military: 1.7,
            shipyard: 1.5,
        },
    }
}

fn military_explosives() {
    #{
        id: "military_explosives",
        name: "Military Explosives",
        description: "High-yield explosives for military use. Regulated.",
        base_price: 350,
        weight: 2,
        legal: true,  // Legal but heavily regulated
        volatility: 0.22,
        contraband_penalty: 25,  // Small penalty if in wrong hands
        category: "military",
        special_effects: ["volatile"],
        icon: "explosive",
        tier: 2,
        abundance: 0.3,
        demand_modifiers: #{
            military: 1.8,
            mining: 1.2,  // Also used in mining
        },
    }
}

// =============================================================================
// Data & Information
// =============================================================================

fn data_cores() {
    #{
        id: "data_cores",
        name: "Data Cores",
        description: "Encrypted data storage devices.",
        base_price: 200,
        weight: 1,
        legal: true,
        volatility: 0.25,
        contraband_penalty: 0,
        category: "data",
        special_effects: [],
        icon: "data",
        tier: 2,
        abundance: 0.5,
        demand_modifiers: #{
            research: 1.4,
            hub: 1.2,
        },
    }
}

fn star_charts() {
    #{
        id: "star_charts",
        name: "Star Charts",
        description: "Navigational data for unexplored regions.",
        base_price: 350,
        weight: 1,
        legal: true,
        volatility: 0.3,
        contraband_penalty: 0,
        category: "data",
        special_effects: ["exploration"],
        icon: "charts",
        tier: 2,
        abundance: 0.3,
        demand_modifiers: #{
            exploration: 1.8,
            military: 1.3,
        },
    }
}

fn military_intel() {
    #{
        id: "military_intel",
        name: "Military Intelligence",
        description: "Classified military information. Very illegal to trade.",
        base_price: 800,
        weight: 1,
        legal: false,
        volatility: 0.4,
        contraband_penalty: 200,
        category: "data",
        special_effects: [],
        icon: "classified",
        tier: 4,
        abundance: 0.1,
        demand_modifiers: #{
            pirate: 1.5,
            enemy_faction: 2.5,
        },
    }
}

// =============================================================================
// Export Function - REQUIRED
// =============================================================================

/// Returns all cargo definitions.
/// This function is called by the ArchetypeRegistry to load cargo types.
fn all_cargo() {
    [
        // Consumables
        fuel(),
        ammunition(),
        energy_cells(),
        medical_supplies(),

        // Industrial
        ore(),
        refined_metals(),
        electronics(),
        construction_materials(),
        machinery(),

        // High value
        rare_earths(),
        luxury_goods(),
        alien_artifacts(),
        quantum_cores(),

        // Food & Agriculture
        food(),
        water(),
        livestock(),

        // Contraband
        stolen_goods(),
        narcotics(),
        counterfeit_credits(),
        slaves(),
        biological_weapons(),

        // Military
        weapons_components(),
        shield_generators(),
        military_explosives(),

        // Data
        data_cores(),
        star_charts(),
        military_intel(),
    ]
}
