// Upgrades Script
//
// Handles ship upgrade purchasing and installation.
// Validates requirements and applies stat bonuses.

// Check if a player can purchase an upgrade
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - upgrade_id: String (upgrade ID from data file)
//
// Returns: Map { can_purchase: bool, reason: String, cost: Map }
fn can_purchase_upgrade(player_id, ship_id, upgrade_id) {
    // Get upgrade definition
    let upgrade_def = get_upgrade_def(upgrade_id);
    if upgrade_def == () {
        return #{
            can_purchase: false,
            reason: "Unknown upgrade: " + upgrade_id,
            cost: ()
        };
    }

    // Get player data
    let player = query_player(player_id);
    if player == () {
        return #{
            can_purchase: false,
            reason: "Player not found",
            cost: ()
        };
    }

    // Get ship data
    let ship = query_ship(ship_id);
    if ship == () {
        return #{
            can_purchase: false,
            reason: "Ship not found",
            cost: ()
        };
    }

    // Check credit cost
    let credits_cost = upgrade_def["credits_cost"];
    if credits_cost == () { credits_cost = 0; }

    let player_credits = get_credits(player_id);
    if player_credits < credits_cost {
        return #{
            can_purchase: false,
            reason: "Insufficient credits. Need " + credits_cost + ", have " + player_credits,
            cost: #{ credits: credits_cost, reputation: 0 }
        };
    }

    // Check reputation cost
    let reputation_cost = upgrade_def["reputation_cost"];
    if reputation_cost == () { reputation_cost = 0; }

    let player_rep = player["reputation"];
    if player_rep == () { player_rep = 0; }

    if player_rep < reputation_cost {
        return #{
            can_purchase: false,
            reason: "Insufficient reputation. Need " + reputation_cost + ", have " + player_rep,
            cost: #{ credits: credits_cost, reputation: reputation_cost }
        };
    }

    // Check ship tier requirement
    let requires = upgrade_def["requires"];
    if requires != () {
        let min_tier = requires["min_ship_tier"];
        if min_tier != () {
            let ship_tier = get_ship_tier(ship_id);
            if ship_tier < min_tier {
                return #{
                    can_purchase: false,
                    reason: "Ship tier too low. Requires tier " + min_tier + " ship",
                    cost: #{ credits: credits_cost, reputation: reputation_cost }
                };
            }
        }
    }

    // Check if slot is available
    let slot_type = upgrade_def["slot_type"];
    if slot_type != () {
        let has_slot = check_upgrade_slot_available(ship_id, slot_type);
        if !has_slot {
            return #{
                can_purchase: false,
                reason: "No available " + slot_type + " slot on ship",
                cost: #{ credits: credits_cost, reputation: reputation_cost }
            };
        }
    }

    #{
        can_purchase: true,
        reason: "Requirements met",
        cost: #{ credits: credits_cost, reputation: reputation_cost }
    }
}

// Purchase and install an upgrade
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - upgrade_id: String (upgrade ID from data file)
//   - slot: String (optional specific slot name)
//
// Returns: Map { success: bool, error: String }
fn purchase_upgrade(player_id, ship_id, upgrade_id, slot) {
    // First check if purchase is allowed
    let check = can_purchase_upgrade(player_id, ship_id, upgrade_id);
    if !check["can_purchase"] {
        send_error(player_id, "Cannot purchase " + upgrade_id + ": " + check["reason"]);
        return #{
            success: false,
            error: check["reason"]
        };
    }

    let cost = check["cost"];
    let upgrade_def = get_upgrade_def(upgrade_id);

    // Deduct credits
    let credits_cost = cost["credits"];
    if credits_cost > 0 {
        if !spend_credits(player_id, credits_cost) {
            return #{
                success: false,
                error: "Failed to deduct credits"
            };
        }
    }

    // Deduct reputation (if applicable - not all systems do this)
    // For now, reputation is just a requirement, not consumed

    // Determine slot to use
    let slot_type = upgrade_def["slot_type"];
    if slot == "" || slot == () {
        slot = find_available_slot(ship_id, slot_type);
    }

    if slot == () || slot == "" {
        // Refund credits
        if credits_cost > 0 {
            add_credits(player_id, credits_cost);
        }
        return #{
            success: false,
            error: "No available slot for upgrade"
        };
    }

    // Install the upgrade
    if install_upgrade(ship_id, upgrade_id, slot) {
        let upgrade_name = upgrade_def["name"];
        if upgrade_name == () { upgrade_name = upgrade_id; }

        send_success(player_id, "Installed " + upgrade_name + " in " + slot + " slot for " + credits_cost + " credits");

        return #{
            success: true,
            error: "",
            slot: slot,
            credits_spent: credits_cost
        };
    }

    // Refund on failure
    if credits_cost > 0 {
        add_credits(player_id, credits_cost);
    }

    #{
        success: false,
        error: "Failed to install upgrade"
    }
}

// Remove an upgrade from a ship (partial refund)
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - slot: String (slot to remove upgrade from)
//
// Returns: Map { success: bool, refund: i64 }
fn sell_upgrade(player_id, ship_id, slot) {
    // Get current upgrades to find what's in the slot
    let upgrades = get_upgrades(ship_id);
    let found_upgrade = ();

    for upgrade in upgrades {
        if upgrade["slot"] == slot {
            found_upgrade = upgrade;
        }
    }

    if found_upgrade == () {
        send_error(player_id, "No upgrade installed in " + slot + " slot");
        return #{
            success: false,
            refund: 0,
            error: "No upgrade in slot"
        };
    }

    let upgrade_id = found_upgrade["upgrade_id"];
    let upgrade_def = get_upgrade_def(upgrade_id);

    // Calculate refund (50% of original price)
    let refund = 0;
    if upgrade_def != () {
        let original_cost = upgrade_def["credits_cost"];
        if original_cost != () {
            refund = original_cost / 2;
        }
    }

    // Remove the upgrade
    if remove_upgrade(ship_id, slot) {
        if refund > 0 {
            add_credits(player_id, refund);
        }

        let upgrade_name = upgrade_def["name"];
        if upgrade_name == () { upgrade_name = upgrade_id; }

        send_notification(player_id, "Removed " + upgrade_name + ". Refund: " + refund + " credits");

        return #{
            success: true,
            refund: refund,
            error: ""
        };
    }

    #{
        success: false,
        refund: 0,
        error: "Failed to remove upgrade"
    }
}

// Get list of available upgrades at a station
//
// Arguments:
//   - station_id: String (station UUID)
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//
// Returns: Array of upgrade info maps
fn get_available_upgrades(station_id, player_id, ship_id) {
    let available = [];

    // All upgrade IDs from data
    let upgrade_ids = [
        "railgun_mk1", "railgun_mk2", "railgun_mk3",
        "missile_launcher_mk1", "missile_launcher_mk2",
        "shield_capacitor", "shield_generator_mk2",
        "armor_plating", "reactive_armor",
        "thruster_upgrade", "afterburner",
        "targeting_computer", "advanced_sensors",
        "cargo_expander", "fuel_tank"
    ];

    for upgrade_id in upgrade_ids {
        let upgrade_def = get_upgrade_def(upgrade_id);
        if upgrade_def != () {
            let check = can_purchase_upgrade(player_id, ship_id, upgrade_id);

            let upgrade_name = upgrade_def["name"];
            if upgrade_name == () { upgrade_name = upgrade_id; }

            let credits_cost = upgrade_def["credits_cost"];
            if credits_cost == () { credits_cost = 0; }

            let reputation_cost = upgrade_def["reputation_cost"];
            if reputation_cost == () { reputation_cost = 0; }

            let slot_type = upgrade_def["slot_type"];
            if slot_type == () { slot_type = "universal"; }

            let tier = upgrade_def["tier"];
            if tier == () { tier = 1; }

            available.push(#{
                id: upgrade_id,
                name: upgrade_name,
                slot_type: slot_type,
                tier: tier,
                credits_cost: credits_cost,
                reputation_cost: reputation_cost,
                can_purchase: check["can_purchase"],
                reason: check["reason"],
                effects: upgrade_def["effects"]
            });
        }
    }

    available
}

// Get ship tier from ship class
fn get_ship_tier(ship_id) {
    let ship = query_ship(ship_id);
    if ship == () { return 1; }

    let ship_class = ship["ship_class"];
    let ship_def = get_ship_def(ship_class);

    if ship_def != () {
        let tier = ship_def["tier"];
        if tier != () {
            return tier;
        }
    }

    // Default tier based on class name
    if ship_class == "patrol_corvette" { return 1; }
    if ship_class == "frigate" { return 2; }
    if ship_class == "destroyer" { return 3; }
    if ship_class == "cruiser" { return 4; }
    if ship_class == "battlecruiser" { return 5; }

    1 // Default
}

// Check if a specific slot type is available
fn check_upgrade_slot_available(ship_id, slot_type) {
    let ship = query_ship(ship_id);
    if ship == () { return false; }

    let ship_class = ship["ship_class"];
    let ship_def = get_ship_def(ship_class);

    if ship_def == () { return false; }

    let slots = ship_def["upgrade_slots"];
    if slots == () { return false; }

    // Check if any slot of this type exists
    let has_slot_type = false;
    for s in slots {
        if s == slot_type || s.starts_with(slot_type) {
            has_slot_type = true;
        }
    }

    if !has_slot_type {
        return false;
    }

    // Check if slot is occupied
    let upgrades = get_upgrades(ship_id);
    let occupied_count = 0;
    let total_slots = 0;

    for s in slots {
        if s == slot_type || s.starts_with(slot_type) {
            total_slots += 1;
        }
    }

    for upgrade in upgrades {
        let upgrade_slot = upgrade["slot"];
        if upgrade_slot == slot_type || upgrade_slot.starts_with(slot_type) {
            occupied_count += 1;
        }
    }

    occupied_count < total_slots
}

// Find first available slot of given type
fn find_available_slot(ship_id, slot_type) {
    let ship = query_ship(ship_id);
    if ship == () { return (); }

    let ship_class = ship["ship_class"];
    let ship_def = get_ship_def(ship_class);

    if ship_def == () { return (); }

    let slots = ship_def["upgrade_slots"];
    if slots == () { return (); }

    let upgrades = get_upgrades(ship_id);
    let occupied = [];

    for upgrade in upgrades {
        occupied.push(upgrade["slot"]);
    }

    // Find first unoccupied slot of matching type
    for s in slots {
        if s == slot_type || s.starts_with(slot_type) {
            let is_occupied = false;
            for o in occupied {
                if o == s {
                    is_occupied = true;
                }
            }
            if !is_occupied {
                return s;
            }
        }
    }

    ()
}

// Calculate total stat bonuses from all upgrades
fn calculate_upgrade_bonuses(ship_id) {
    let upgrades = get_upgrades(ship_id);
    let bonuses = #{
        attack_bonus: 0.0,
        defense_bonus: 0.0,
        speed_bonus: 0.0,
        shield_bonus: 0.0,
        shield_regen: 0.0,
        sensor_bonus: 0.0,
        accuracy_bonus: 0.0,
        crit_bonus: 0.0,
        cargo_bonus: 0
    };

    for upgrade in upgrades {
        let upgrade_id = upgrade["upgrade_id"];
        let upgrade_def = get_upgrade_def(upgrade_id);

        if upgrade_def != () {
            let effects = upgrade_def["effects"];
            if effects != () {
                // Accumulate all effects
                if effects["attack_bonus"] != () {
                    bonuses["attack_bonus"] += effects["attack_bonus"];
                }
                if effects["defense_bonus"] != () {
                    bonuses["defense_bonus"] += effects["defense_bonus"];
                }
                if effects["speed_bonus"] != () {
                    bonuses["speed_bonus"] += effects["speed_bonus"];
                }
                if effects["shield_bonus"] != () {
                    bonuses["shield_bonus"] += effects["shield_bonus"];
                }
                if effects["shield_regen"] != () {
                    bonuses["shield_regen"] += effects["shield_regen"];
                }
                if effects["sensor_bonus"] != () {
                    bonuses["sensor_bonus"] += effects["sensor_bonus"];
                }
                if effects["accuracy_bonus"] != () {
                    bonuses["accuracy_bonus"] += effects["accuracy_bonus"];
                }
                if effects["crit_bonus"] != () {
                    bonuses["crit_bonus"] += effects["crit_bonus"];
                }
                if effects["cargo_bonus"] != () {
                    bonuses["cargo_bonus"] += effects["cargo_bonus"];
                }
            }
        }
    }

    bonuses
}

// Get upgrade comparison (new vs current)
fn compare_upgrade(ship_id, new_upgrade_id, slot) {
    let new_def = get_upgrade_def(new_upgrade_id);
    if new_def == () {
        return #{ error: "Unknown upgrade" };
    }

    let new_effects = new_def["effects"];
    if new_effects == () { new_effects = #{}; }

    // Check if slot has existing upgrade
    let upgrades = get_upgrades(ship_id);
    let current_upgrade = ();

    for upgrade in upgrades {
        if upgrade["slot"] == slot {
            current_upgrade = upgrade;
        }
    }

    let current_effects = #{};
    if current_upgrade != () {
        let current_def = get_upgrade_def(current_upgrade["upgrade_id"]);
        if current_def != () {
            current_effects = current_def["effects"];
            if current_effects == () { current_effects = #{}; }
        }
    }

    // Calculate differences
    let diff = #{};
    let stats = ["attack_bonus", "defense_bonus", "speed_bonus", "shield_bonus", "accuracy_bonus", "crit_bonus"];

    for stat in stats {
        let new_val = new_effects[stat];
        if new_val == () { new_val = 0.0; }

        let old_val = current_effects[stat];
        if old_val == () { old_val = 0.0; }

        let change = new_val - old_val;
        if change != 0.0 {
            diff[stat] = #{
                old_value: old_val,
                new_value: new_val,
                change: change
            };
        }
    }

    #{
        new_upgrade: new_def["name"],
        current_upgrade: if current_upgrade != () { current_upgrade["upgrade_id"] } else { "None" },
        stat_changes: diff,
        is_upgrade: true  // Could calculate if net positive
    }
}
