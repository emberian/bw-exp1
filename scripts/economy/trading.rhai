// Trading Script
//
// Handles buy/sell transactions at stations.
// All price calculations and validation logic lives here.

// Execute a buy transaction
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - station_id: String (station UUID)
//   - cargo_type: String (cargo type ID)
//   - quantity: i64 (amount to buy)
//
// Returns: Map { success: bool, error: String, credits_spent: i64, quantity_bought: i64 }
fn execute_buy(player_id, ship_id, station_id, cargo_type, quantity) {
    // Get cargo definition from archetype registry
    let cargo_def = get_cargo_def(cargo_type);
    if cargo_def == () {
        return #{
            success: false,
            error: "Unknown cargo type: " + cargo_type,
            credits_spent: 0,
            quantity_bought: 0
        };
    }

    // Calculate price with station markup
    let base_price = cargo_def["base_price"];
    if base_price == () { base_price = 100; }

    let buy_price = calculate_buy_price(station_id, cargo_type, base_price);
    let total_cost = buy_price * quantity;

    // Check player credits
    let credits = get_credits(player_id);
    if credits < total_cost {
        // Calculate how many they can afford
        let affordable = credits / buy_price;
        if affordable <= 0 {
            return #{
                success: false,
                error: "Insufficient credits. Need " + total_cost + ", have " + credits,
                credits_spent: 0,
                quantity_bought: 0
            };
        }
        // Partial buy
        quantity = affordable;
        total_cost = buy_price * quantity;
    }

    // Check cargo capacity
    let capacity = get_cargo_capacity(ship_id);
    let used = get_cargo_used(ship_id);
    let available_space = capacity - used;

    let weight = cargo_def["weight"];
    if weight == () { weight = 1; }
    let space_needed = weight * quantity;

    if space_needed > available_space {
        // Calculate how many will fit
        let can_fit = available_space / weight;
        if can_fit <= 0 {
            return #{
                success: false,
                error: "Cargo hold full. Need " + space_needed + " units, have " + available_space,
                credits_spent: 0,
                quantity_bought: 0
            };
        }
        // Partial buy
        quantity = can_fit;
        total_cost = buy_price * quantity;
    }

    // Execute the transaction
    if spend_credits(player_id, total_cost) {
        if add_cargo(ship_id, cargo_type, quantity, buy_price) {
            // Success notification
            let cargo_name = cargo_def["name"];
            if cargo_name == () { cargo_name = cargo_type; }
            send_success(player_id, "Purchased " + quantity + "x " + cargo_name + " for " + total_cost + " credits");

            return #{
                success: true,
                error: "",
                credits_spent: total_cost,
                quantity_bought: quantity
            };
        } else {
            // Refund if cargo add failed
            add_credits(player_id, total_cost);
            return #{
                success: false,
                error: "Failed to add cargo to ship",
                credits_spent: 0,
                quantity_bought: 0
            };
        }
    }

    #{
        success: false,
        error: "Failed to process payment",
        credits_spent: 0,
        quantity_bought: 0
    }
}

// Execute a sell transaction
//
// Arguments:
//   - player_id: String (player UUID)
//   - ship_id: String (ship UUID)
//   - station_id: String (station UUID)
//   - cargo_type: String (cargo type ID)
//   - quantity: i64 (amount to sell)
//
// Returns: Map { success: bool, error: String, credits_gained: i64, quantity_sold: i64 }
fn execute_sell(player_id, ship_id, station_id, cargo_type, quantity) {
    // Get cargo definition from archetype registry
    let cargo_def = get_cargo_def(cargo_type);
    if cargo_def == () {
        return #{
            success: false,
            error: "Unknown cargo type: " + cargo_type,
            credits_gained: 0,
            quantity_sold: 0
        };
    }

    // Check how much cargo the player has
    let cargo = get_cargo(ship_id);
    let have_quantity = 0;
    let purchase_price = 0;

    for item in cargo {
        if item["type"] == cargo_type {
            have_quantity = item["quantity"];
            purchase_price = item["price"];
        }
    }

    if have_quantity <= 0 {
        return #{
            success: false,
            error: "You don't have any " + cargo_type,
            credits_gained: 0,
            quantity_sold: 0
        };
    }

    // Clamp quantity to what player has
    if quantity > have_quantity {
        quantity = have_quantity;
    }

    // Check for contraband
    let is_legal = cargo_def["legal"];
    if is_legal == () { is_legal = true; }

    if !is_legal {
        // Contraband check - chance of being caught
        let result = check_contraband_sale(player_id, ship_id, station_id, cargo_type, quantity);
        if result["caught"] {
            return result;
        }
    }

    // Calculate sell price
    let base_price = cargo_def["base_price"];
    if base_price == () { base_price = 100; }

    let sell_price = calculate_sell_price(station_id, cargo_type, base_price);
    let total_value = sell_price * quantity;

    // Calculate profit/loss
    let cost_basis = purchase_price * quantity;
    let profit = total_value - cost_basis;

    // Execute the transaction
    if remove_cargo(ship_id, cargo_type, quantity) {
        add_credits(player_id, total_value);

        // Success notification with profit info
        let cargo_name = cargo_def["name"];
        if cargo_name == () { cargo_name = cargo_type; }

        let profit_msg = "";
        if profit > 0 {
            profit_msg = " (+" + profit + " profit)";
        } else if profit < 0 {
            profit_msg = " (" + profit + " loss)";
        }

        send_success(player_id, "Sold " + quantity + "x " + cargo_name + " for " + total_value + " credits" + profit_msg);

        return #{
            success: true,
            error: "",
            credits_gained: total_value,
            quantity_sold: quantity,
            profit: profit
        };
    }

    #{
        success: false,
        error: "Failed to remove cargo from ship",
        credits_gained: 0,
        quantity_sold: 0
    }
}

// Calculate buy price at a station (with markup)
fn calculate_buy_price(station_id, cargo_type, base_price) {
    // Stations sell at markup
    let markup = 1.15; // 15% markup

    // TODO: Could add station-specific pricing, supply/demand, etc.
    let volatility = get_cargo_volatility(cargo_type);
    let variance = 1.0 + (rand() - 0.5) * volatility;

    let final_price = base_price * markup * variance;
    if final_price < 1 { final_price = 1; }

    floor(final_price)
}

// Calculate sell price at a station (with markdown)
fn calculate_sell_price(station_id, cargo_type, base_price) {
    // Stations buy at discount
    let discount = 0.85; // 15% below base

    // TODO: Could add station-specific pricing, supply/demand, etc.
    let volatility = get_cargo_volatility(cargo_type);
    let variance = 1.0 + (rand() - 0.5) * volatility;

    let final_price = base_price * discount * variance;
    if final_price < 1 { final_price = 1; }

    floor(final_price)
}

// Get cargo price volatility
fn get_cargo_volatility(cargo_type) {
    let cargo_def = get_cargo_def(cargo_type);
    if cargo_def != () {
        let vol = cargo_def["volatility"];
        if vol != () {
            return vol;
        }
    }
    0.1 // Default 10% volatility
}

// Check for contraband sale detection
fn check_contraband_sale(player_id, ship_id, station_id, cargo_type, quantity) {
    // 20% base chance of being caught
    let catch_chance = 0.2;

    // Higher quantities increase chance
    if quantity > 10 { catch_chance += 0.1; }
    if quantity > 50 { catch_chance += 0.2; }

    if rand() < catch_chance {
        // Caught!
        let cargo_def = get_cargo_def(cargo_type);
        let penalty = cargo_def["contraband_penalty"];
        if penalty == () { penalty = 50; }

        // Apply reputation penalty
        modify_player(player_id, #{ reputation_delta: -penalty });

        // Confiscate the cargo
        remove_cargo(ship_id, cargo_type, quantity);

        let cargo_name = cargo_def["name"];
        if cargo_name == () { cargo_name = cargo_type; }

        send_error(player_id, "Contraband detected! Your " + cargo_name + " has been confiscated. Reputation penalty: -" + penalty);

        return #{
            success: false,
            error: "Contraband detected and confiscated",
            credits_gained: 0,
            quantity_sold: 0,
            caught: true,
            reputation_penalty: penalty
        };
    }

    #{ caught: false }
}

// Get player's current ship ID (helper)
fn get_player_ship_id(player_id) {
    let player = query_player(player_id);
    if player != () {
        let ship_id = player["ship_id"];
        if ship_id != () {
            return ship_id;
        }
    }
    ""
}

// Get market prices for display
//
// Arguments:
//   - station_id: String (station UUID)
//
// Returns: Array of price info maps
fn get_market_prices(station_id) {
    let prices = [];

    // Get all available cargo types from archetype registry
    let cargo_defs = all_cargo_types();

    for cargo_def in cargo_defs {
        let cargo_type = cargo_def["id"];
        let base_price = cargo_def["base_price"];
        if base_price == () { base_price = 100; }

        let buy_price = calculate_buy_price(station_id, cargo_type, base_price);
        let sell_price = calculate_sell_price(station_id, cargo_type, base_price);

        let is_legal = cargo_def["legal"];
        if is_legal == () { is_legal = true; }

        prices.push(#{
            cargo_type: cargo_type,
            name: cargo_def["name"],
            buy_price: buy_price,
            sell_price: sell_price,
            legal: is_legal,
            weight: cargo_def["weight"]
        });
    }

    prices
}

// Calculate potential profit for a trade route
fn calculate_trade_profit(buy_station_id, sell_station_id, cargo_type, quantity) {
    let cargo_def = get_cargo_def(cargo_type);
    if cargo_def == () {
        return #{ error: "Unknown cargo type" };
    }

    let base_price = cargo_def["base_price"];
    if base_price == () { base_price = 100; }

    let buy_price = calculate_buy_price(buy_station_id, cargo_type, base_price);
    let sell_price = calculate_sell_price(sell_station_id, cargo_type, base_price);

    let cost = buy_price * quantity;
    let revenue = sell_price * quantity;
    let profit = revenue - cost;
    let margin = if cost > 0 { (profit * 100) / cost } else { 0 };

    #{
        cargo_type: cargo_type,
        quantity: quantity,
        buy_price_per_unit: buy_price,
        sell_price_per_unit: sell_price,
        total_cost: cost,
        total_revenue: revenue,
        profit: profit,
        margin_percent: margin
    }
}

// Helper: floor function
fn floor(x) {
    let i = x;
    if i < 0 {
        i - 1
    } else {
        i - (i % 1)
    }
}
