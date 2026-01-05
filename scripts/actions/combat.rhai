// Combat action handlers
//
// Handles engage, disengage, and weapon firing.

fn init() {
    register_action_with_requirements("engage_target", "handle_engage_target", [
        "authenticated",
        #{ ship_status: ["idle", "moving"] }
    ]);

    register_action_with_requirements("disengage_combat", "handle_disengage_combat", [
        "authenticated"
    ]);

    register_action_with_requirements("fire_weapon", "handle_fire_weapon", [
        "authenticated",
        #{ ship_status: ["in_combat"] }
    ]);

    log_info("Combat actions registered");
}

// Engage a target
// params: { target_id: uuid }
fn handle_engage_target(ctx, params) {
    let target_id = params["target_id"];
    if target_id == () {
        return #{ success: false, error: "target_id required" };
    }

    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];
    let sector_id = ctx["sector_id"];

    // Can't target self
    if target_id == ship_id {
        return #{ success: false, error: "Cannot target yourself" };
    }

    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Check current status
    if ship["status"] == "docked" {
        return #{ success: false, error: "Cannot engage while docked" };
    }
    if ship["status"] == "in_transit" {
        return #{ success: false, error: "Cannot engage while in transit" };
    }
    if ship["status"] == "in_combat" {
        return #{ success: false, error: "Already in combat" };
    }

    // Query target
    let target = query_ship(target_id);
    if target == () {
        return #{ success: false, error: "Target not found" };
    }

    // Check target is in same sector
    if target["sector_id"] != sector_id {
        return #{ success: false, error: "Target is not in this sector" };
    }

    // Check target is not docked
    if target["status"] == "docked" {
        return #{ success: false, error: "Cannot attack docked ships" };
    }

    // Check distance (must be within weapon range, default 500)
    let dist = distance_between(ship["position"], target["position"]);
    let max_range = 500.0;
    if dist > max_range {
        return #{
            success: false,
            error: "Target out of range (" + dist.to_int() + " units, max " + max_range.to_int() + ")"
        };
    }

    // Check if target is friendly (same faction)
    let ship_faction = ship["faction_id"];
    let target_faction = target["faction_id"];
    if ship_faction != () && target_faction != () && ship_faction == target_faction {
        // Allow with warning for faction ships
        send_notification(player_id, "Warning: Engaging friendly vessel!");
    }

    // Create combat engagement
    let engagement_id = create_combat_engagement(ship_id, target_id, sector_id);

    // Update both ships to in_combat status
    modify_ship(ship_id, #{
        status: "in_combat",
        combat_target: target_id,
        engagement_id: engagement_id
    });

    // If target is NPC, also put them in combat
    if target["is_player"] == false {
        modify_ship(target_id, #{
            status: "in_combat",
            combat_target: ship_id,
            engagement_id: engagement_id
        });
    }

    send_notification(player_id, "Engaging " + target["name"]);

    emit_event("combat_started", #{
        engagement_id: engagement_id,
        attacker_id: ship_id,
        defender_id: target_id,
        sector_id: sector_id
    });

    #{
        success: true,
        data: #{
            engagement_id: engagement_id,
            target_name: target["name"],
            target_hull: target["hull"],
            target_shields: target["shields"]
        }
    }
}

// Disengage from combat
fn handle_disengage_combat(ctx, params) {
    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];

    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    if ship["status"] != "in_combat" {
        return #{ success: false, error: "Not in combat" };
    }

    // Get engagement
    let engagement_id = ship["engagement_id"];
    if engagement_id == () {
        // No engagement ID, just clear status
        modify_ship(ship_id, #{
            status: "idle",
            combat_target: (),
            engagement_id: ()
        });
        return #{ success: true };
    }

    // Check if disengage is allowed (e.g., combat round > 3, or enemy destroyed)
    let engagement = query_combat_engagement(engagement_id);
    if engagement != () {
        let round = engagement["round"];
        if round < 3 {
            return #{
                success: false,
                error: `Cannot disengage until round 3 (currently round ${round})`
            };
        }
    }

    // Disengage - there's a chance the enemy gets a parting shot
    let escape_roll = random_float();
    let escape_chance = 0.7; // 70% base escape chance

    // Modify by ship speed/evasion if available
    let ship_class = ship["ship_class"];
    if ship_class == "Scout" || ship_class == "Interceptor" {
        escape_chance = 0.9;
    } else if ship_class == "Cruiser" || ship_class == "Battleship" {
        escape_chance = 0.5;
    }

    if escape_roll > escape_chance {
        // Failed escape - take some damage
        let current_hull = ship["hull"];
        let damage = random_int(5, 15);
        modify_ship(ship_id, #{
            hull: current_hull - damage
        });
        send_notification(player_id, `Escape attempt failed! Took ${damage} damage.`);
        return #{
            success: false,
            error: "Failed to disengage - enemy landed a hit!",
            data: #{ damage_taken: damage }
        };
    }

    // Successful disengage
    modify_ship(ship_id, #{
        status: "idle",
        combat_target: (),
        engagement_id: ()
    });

    // End engagement if exists
    if engagement_id != () {
        end_combat_engagement(engagement_id, "disengaged");
    }

    send_notification(player_id, "Successfully disengaged from combat");

    emit_event("combat_disengaged", #{
        player_id: player_id,
        ship_id: ship_id,
        engagement_id: engagement_id
    });

    #{ success: true }
}

// Fire a weapon at target
// params: { weapon_index: int, target_id: uuid }
fn handle_fire_weapon(ctx, params) {
    let weapon_index = params["weapon_index"];
    let target_id = params["target_id"];

    if weapon_index == () {
        return #{ success: false, error: "weapon_index required" };
    }
    if target_id == () {
        return #{ success: false, error: "target_id required" };
    }

    let ship_id = ctx["ship_id"];
    let player_id = ctx["player_id"];

    let ship = query_ship(ship_id);
    if ship == () {
        return #{ success: false, error: "Ship not found" };
    }

    // Verify in combat
    if ship["status"] != "in_combat" {
        return #{ success: false, error: "Not in combat" };
    }

    // Get weapon
    let weapons = ship["weapons"];
    if weapons == () || weapon_index >= weapons.len() {
        return #{ success: false, error: "Invalid weapon index" };
    }

    let weapon = weapons[weapon_index];

    // Check weapon cooldown
    let cooldown = weapon["cooldown_remaining"];
    if cooldown != () && cooldown > 0 {
        return #{ success: false, error: `Weapon on cooldown (${cooldown} ticks remaining)` };
    }

    // Check ammunition
    let ammo_cost = weapon["ammo_cost"];
    if ammo_cost == () { ammo_cost = 1.0; }

    let current_ammo = ship["ammunition"];
    if current_ammo < ammo_cost {
        return #{ success: false, error: "Insufficient ammunition" };
    }

    // Query target
    let target = query_ship(target_id);
    if target == () {
        return #{ success: false, error: "Target not found" };
    }

    // Calculate hit chance
    let base_accuracy = weapon["accuracy"];
    if base_accuracy == () { base_accuracy = 0.7; }

    // Modify by distance
    let dist = distance_between(ship["position"], target["position"]);
    let weapon_range = weapon["range"];
    if weapon_range == () { weapon_range = 500.0; }

    if dist > weapon_range {
        return #{ success: false, error: "Target out of weapon range" };
    }

    let range_penalty = (dist / weapon_range) * 0.3;
    let hit_chance = base_accuracy - range_penalty;

    // Roll for hit
    let roll = random_float();
    let hit = roll <= hit_chance;

    // Consume ammunition
    modify_ship(ship_id, #{
        ammunition: current_ammo - ammo_cost
    });

    // Set weapon cooldown
    set_weapon_cooldown(ship_id, weapon_index, weapon["cooldown"]);

    if !hit {
        send_notification(player_id, weapon["name"] + " missed!");
        return #{
            success: true,
            data: #{
                hit: false,
                weapon: weapon["name"]
            }
        };
    }

    // Calculate damage
    let base_damage = weapon["damage"];
    if base_damage == () { base_damage = 10.0; }

    // Apply damage to shields first, then hull
    let target_shields = target["shields"];
    let target_hull = target["hull"];
    let shield_damage = 0.0;
    let hull_damage = 0.0;

    if target_shields > 0 {
        shield_damage = base_damage.min(target_shields);
        let remaining = base_damage - shield_damage;
        if remaining > 0 {
            hull_damage = remaining * 0.5; // Shields absorb some bleedthrough
        }
    } else {
        hull_damage = base_damage;
    }

    // Apply damage
    modify_ship(target_id, #{
        shields: target_shields - shield_damage,
        hull: target_hull - hull_damage
    });

    send_notification(player_id, weapon["name"] + " hit! Dealt " + hull_damage.to_int() + " hull damage");

    emit_event("weapon_fired", #{
        attacker_id: ship_id,
        target_id: target_id,
        weapon_name: weapon["name"],
        damage: hull_damage,
        shield_damage: shield_damage
    });

    // Check if target destroyed
    if target_hull - hull_damage <= 0 {
        emit_event("ship_destroyed", #{
            ship_id: target_id,
            destroyed_by: ship_id
        });
    }

    #{
        success: true,
        data: #{
            hit: true,
            weapon: weapon["name"],
            hull_damage: hull_damage,
            shield_damage: shield_damage,
            target_hull_remaining: target_hull - hull_damage,
            target_shields_remaining: target_shields - shield_damage
        }
    }
}
