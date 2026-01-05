// NPC Ship Name Generation
//
// Generates names for NPC ships based on their class.
//
// Context (ctx) contains:
//   - ship_class: String (e.g., "PirateRaider", "Freighter")
//   - faction_tag: String or ()
//
// Returns: String (the ship name)

// Name prefixes by ship class
fn get_prefixes(ship_class) {
    switch ship_class {
        "Freighter" => ["Cargo", "Hauler", "Bulk", "Star", "Trade"],
        "Transport" => ["Transit", "Shuttle", "Ferry", "Express", "Liner"],
        "MiningVessel" => ["Drill", "Excavator", "Miner", "Prospect", "Deep"],
        "PirateRaider" => ["Black", "Shadow", "Red", "Rogue", "Swift"],
        "PirateFrigate" => ["Dread", "Crimson", "Iron", "Storm", "Void"],
        "TerroristBomber" => ["Zealot", "Martyr", "Flame", "Fury", "End"],
        "SeraSwarm" => ["Swarm", "Cluster", "Mass", "Wave", "Tide"],
        "SeraHunter" => ["Hunter", "Stalker", "Reaper", "Slayer", "Doom"],
        "DroneHarvester" => ["Harvester", "Collector", "Gatherer", "Scavenger", "Reaper"],
        "DroneSwarm" => ["Drone", "Unit", "Cluster", "Wing", "Pack"],
        _ => ["Ship"]
    }
}

// Name suffixes by ship class
fn get_suffixes(ship_class) {
    switch ship_class {
        "Freighter" | "Transport" | "MiningVessel" =>
            ["I", "II", "III", "IV", "V", "A", "B", "C", "Prime", "Delta"],
        "PirateRaider" | "PirateFrigate" =>
            ["Fang", "Claw", "Blade", "Edge", "Thorn", "Hook", "Spike", "Doom"],
        _ =>
            ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta", "Eta", "Theta"]
    }
}

// Pick a random element from an array
fn random_pick(arr) {
    let idx = floor(rand() * arr.len());
    if idx >= arr.len() {
        idx = arr.len() - 1;
    }
    arr[idx]
}

// Main entry point: generate a name for an NPC ship
fn generate_name(ctx) {
    let ship_class = ctx["ship_class"];

    let prefixes = get_prefixes(ship_class);
    let suffixes = get_suffixes(ship_class);

    let prefix = random_pick(prefixes);
    let suffix = random_pick(suffixes);
    let num = floor(rand() * 900.0) + 100;  // 100-999

    `${prefix} ${suffix} ${num}`
}
