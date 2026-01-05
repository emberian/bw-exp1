// NPC Ship Class and Faction Selection
//
// This script determines which ship class to spawn and what faction it belongs to.
// Called by the server when spawning a new NPC.
//
// Context (ctx) contains:
//   - danger_level: String ("Safe", "Moderate", "Dangerous", "Hostile")
//   - traffic_density: String ("Sparse", "Light", "Moderate", "Heavy", "Congested")
//   - current_npc_count: i64
//   - sector_id: String (UUID)
//
// Returns a map with:
//   - ship_class: String (e.g., "PirateRaider", "Freighter")
//   - faction_tag: String or () for no faction

// Ship class weights based on traffic density
fn civilian_weights(traffic) {
    switch traffic {
        "Sparse" => [
            ["Freighter", 5.0],
            ["MiningVessel", 10.0]
        ],
        "Light" => [
            ["Freighter", 15.0],
            ["Transport", 10.0],
            ["MiningVessel", 10.0]
        ],
        "Moderate" => [
            ["Freighter", 25.0],
            ["Transport", 20.0],
            ["MiningVessel", 5.0]
        ],
        "Heavy" | "Congested" => [
            ["Freighter", 35.0],
            ["Transport", 30.0]
        ],
        _ => [
            ["Freighter", 10.0]
        ]
    }
}

// Hostile ship weights based on danger level
fn hostile_weights(danger) {
    switch danger {
        "Safe" => [
            ["PirateRaider", 2.0]
        ],
        "Moderate" => [
            ["PirateRaider", 15.0],
            ["PirateFrigate", 5.0],
            ["DroneSwarm", 5.0]
        ],
        "Dangerous" => [
            ["PirateRaider", 20.0],
            ["PirateFrigate", 15.0],
            ["TerroristBomber", 5.0],
            ["DroneHarvester", 10.0],
            ["DroneSwarm", 10.0],
            ["SeraSwarm", 5.0]
        ],
        "Hostile" => [
            ["PirateRaider", 10.0],
            ["PirateFrigate", 20.0],
            ["TerroristBomber", 10.0],
            ["DroneHarvester", 15.0],
            ["DroneSwarm", 15.0],
            ["SeraSwarm", 15.0],
            ["SeraHunter", 10.0]
        ],
        _ => []
    }
}

// Select faction for a ship class
fn select_faction(ship_class) {
    switch ship_class {
        "Freighter" | "Transport" | "MiningVessel" => "FORGEBORN",
        "PirateRaider" | "PirateFrigate" => "PIRATES",
        "TerroristBomber" => (),  // Terrorists are unaffiliated
        "SeraSwarm" | "SeraHunter" => "SERA",
        "DroneHarvester" | "DroneSwarm" => "DRONES",
        _ => ()
    }
}

// Weighted random selection from a list of [item, weight] pairs
fn weighted_select(weights) {
    let total = 0.0;
    for w in weights {
        total += w[1];
    }

    if total <= 0.0 {
        return "Freighter";  // Fallback
    }

    let roll = rand() * total;
    let accumulated = 0.0;

    for w in weights {
        accumulated += w[1];
        if roll < accumulated {
            return w[0];
        }
    }

    // Fallback (shouldn't reach here)
    weights[0][0]
}

// Main entry point: select which NPC class to spawn
fn select_npc_class(ctx) {
    let danger = ctx["danger_level"];
    let traffic = ctx["traffic_density"];

    // Build combined weight list
    let weights = [];

    // Add civilian ships based on traffic
    for w in civilian_weights(traffic) {
        weights.push(w);
    }

    // Add hostile ships based on danger
    for w in hostile_weights(danger) {
        weights.push(w);
    }

    // Select ship class
    let ship_class = weighted_select(weights);

    // Select faction for this class
    let faction_tag = select_faction(ship_class);

    #{
        ship_class: ship_class,
        faction_tag: faction_tag
    }
}
