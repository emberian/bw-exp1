 6. Implementation Phases

 Phase 1: Primitive Layer

 1. Create #[export_module] bindings for combat, movement, spawning
 2. Implement ShipView, SectorView, EngagementView facades
 3. Move calculate_attack to scripts/systems/combat.rhai
 4. Move calculate_fuel_cost to scripts/systems/movement.rhai

 Phase 2: Entity Archetypes

 1. Create scripts/definitions/ship_classes.rhai
 2. Modify Ship struct to use class_id: String instead of ShipClass enum
 3. Archetype loader that reads definitions at startup (and hot-reload)
 4. Same for weapons, locations

 Phase 3: System Registration

 1. System registry that discovers register() functions in scripts
 2. Event routing to script systems
 3. Priority ordering for system execution

 Phase 4: Existing Features (from previous plan)

 1. Behavior Trees + Utility AI
 2. Script persistence (save_state/load_state)
 3. Property watches

 Phase 5: Admin Tools

 1. In-game console for testing scripts
 2. Archetype browser/editor
 3. Live formula tuning

 ---
 Critical Files

 | File                            | Changes                                        |
 |---------------------------------|------------------------------------------------|
 | bw-core/src/models/ship.rs      | class_id: String instead of enum, stats struct |
 | bw-core/src/systems/combat.rs   | Becomes thin wrapper calling scripts           |
 | bw-core/src/systems/movement.rs | Same                                           |
 | bw-scripting/src/bindings/      | New #[export_module] primitives                |
 | bw-scripting/src/views/         | New facade types (ShipView, etc.)              |
 | New: scripts/definitions/*.rhai | Entity archetypes                              |
 | New: scripts/systems/*.rhai     | Game formulas and logic                        |

 ---
 Example: Admin Adds a Ship Class

 No Rust changes. Admin creates/edits:

 // scripts/definitions/ship_classes.rhai

 fn dreadnought() {
     #{
         id: "dreadnought",
         name: "Dreadnought",
         description: "Massive capital ship. Slow but devastating.",
         stats: #{
             attack: 150.0,
             defense: 120.0,
             speed: 20.0,
             shield_capacity: 300.0,
             sensor_range: 400.0,
             cargo_capacity: 1000,
             fuel_per_sector: 30.0,
         },
         weapons: [
             #{ type: "heavy_railgun", damage: 80.0, accuracy: 0.7, ammo_cost: 5.0 },
             #{ type: "heavy_railgun", damage: 80.0, accuracy: 0.7, ammo_cost: 5.0 },
             #{ type: "missile_battery", damage: 120.0, accuracy: 0.5, ammo_cost: 10.0 },
             #{ type: "point_defense", damage: 15.0, accuracy: 0.95, ammo_cost: 0.5 },
         ],
         behaviors: ["capital_ship", "combat", "flagship"],
         is_player_class: true,
         unlock_requirement: #{ fame: 1000, reputation: 500 },
     }
 }

 // Add to the list
 fn all_ship_classes() {
     [
         patrol_corvette(), frigate(), destroyer(), cruiser(), carrier(),
         dreadnought(),  // Just add it here
     ]
 }
