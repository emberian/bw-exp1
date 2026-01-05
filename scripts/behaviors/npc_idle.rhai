// NPC Idle/Patrol Behavior
// Simple behavior for spawned NPCs - patrols randomly and reacts to nearby players

// Called when behavior is attached
fn on_spawn(ctx) {
    let local = ctx["local_data"];

    // Initialize patrol state
    local["state"] = "idle";
    local["idle_ticks"] = 0;
    local["max_idle_ticks"] = rand_int(50, 150); // 5-15 seconds idle before moving
    local["target_x"] = 0.0;
    local["target_y"] = 0.0;

    #{ local_data: local }
}

// Called each tick (10x per second)
fn on_update(ctx) {
    let entity_id = ctx["entity_id"];
    let sector_id = ctx["sector_id"];
    let local = ctx["local_data"];
    let delta = ctx["delta_time"];

    // Get our ship info (using snapshot API)
    let ship = query_ship(entity_id);
    if ship == () {
        return #{ local_data: local };
    }

    let state = local["state"];

    if state == "idle" {
        // Increment idle counter
        local["idle_ticks"] = local["idle_ticks"] + 1;

        // After idle period, pick a new destination
        if local["idle_ticks"] >= local["max_idle_ticks"] {
            // Pick random position within sector bounds
            let pos = random_position(-400.0, 400.0, -400.0, 400.0);
            local["target_x"] = pos["x"];
            local["target_y"] = pos["y"];
            local["state"] = "moving";
            local["idle_ticks"] = 0;
            local["max_idle_ticks"] = rand_int(50, 150);
        }
    } else if state == "moving" {
        // Move towards target (using nested position from snapshot)
        let pos = ship["position"];
        let current_x = pos["x"];
        let current_y = pos["y"];
        let target_x = local["target_x"];
        let target_y = local["target_y"];

        let dx = target_x - current_x;
        let dy = target_y - current_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 20.0 {
            // Arrived at destination
            local["state"] = "idle";
        } else {
            // Move towards target
            let speed = ship["speed"] * delta * 50.0;
            if speed > dist {
                speed = dist;
            }

            let move_x = (dx / dist) * speed;
            let move_y = (dy / dist) * speed;

            // Apply movement mutation
            modify_ship(entity_id, #{
                position: #{
                    x: current_x + move_x,
                    y: current_y + move_y,
                    z: pos["z"]
                }
            });
        }
    }

    #{ local_data: local }
}

// Called when behavior is detached
fn on_destroy(ctx) {
    // Nothing to clean up
}
