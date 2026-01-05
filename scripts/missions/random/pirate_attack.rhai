// Pirate Attack Mission Script
// A civilian freighter is under attack by pirates.
//
// States: start -> encounter -> combat|resolution -> complete

// Called when mission starts
fn on_start(ctx) {
    let pirate_count = rand_int(1, 3);

    // Create narrative based on pirate count
    let narrative = if pirate_count == 1 {
        "A distress signal crackles through your comms. A civilian freighter reports a lone pirate raider on intercept course."
    } else {
        "A distress signal crackles through your comms. A civilian freighter is under attack by " + pirate_count + " pirate vessels. They won't last long without help."
    };

    // Build choice list
    let choices = [];

    // Always available: Direct attack
    choices.push(create_choice("attack", "Engage the pirates directly"));

    // Experience-gated: Deception
    if meets_requirement(ctx, "MinExperience", 50) {
        choices.push(create_choice("deceive", "Broadcast a fake fleet signature to scare them off"));
    } else {
        choices.push(create_choice_with_req("deceive", "Broadcast a fake fleet signature", "MinExperience", 50));
    }

    // Fame-gated: Negotiation
    if meets_requirement(ctx, "MinFame", 30) {
        choices.push(create_choice("negotiate", "Hail the pirates and demand they stand down"));
    } else {
        choices.push(create_choice_with_req("negotiate", "Demand they stand down", "MinFame", 30));
    }

    // Always available: Abandon
    choices.push(create_choice("abandon", "Let them take the cargo, focus on saving the crew"));

    // Store mission data
    let data = #{
        pirate_count: pirate_count,
        freighter_saved: false,
        cargo_saved: false
    };

    create_choice_result("encounter", narrative, choices)
}

// Called when player makes a choice
fn on_choice(ctx, choice_id) {
    let data = get_mission_data(ctx);
    let pirate_count = data["pirate_count"];

    if choice_id == "attack" {
        handle_attack(ctx, pirate_count)
    } else if choice_id == "deceive" {
        handle_deception(ctx, pirate_count)
    } else if choice_id == "negotiate" {
        handle_negotiation(ctx, pirate_count)
    } else if choice_id == "abandon" {
        handle_abandon(ctx, pirate_count)
    } else {
        create_failure_result(10, "Unknown choice made. Mission failed.")
    }
}

fn handle_attack(ctx, pirate_count) {
    // Start combat
    let narrative = "You power up weapons and move to intercept. The pirates turn to face you - ";
    narrative += if pirate_count == 1 {
        "a single raider, aggressive but outgunned."
    } else {
        pirate_count + " hostiles forming up. This won't be easy."
    };

    create_combat_result(narrative)
}

fn handle_deception(ctx, pirate_count) {
    let experience = get_crew_experience(ctx);

    // Success chance based on experience
    let base_chance = 0.4;
    let success_chance = experience_success_chance(base_chance, experience);

    if rand() < success_chance {
        // Success!
        let narrative = "Your crew broadcasts convincing fleet signatures. The pirates' sensors light up with phantom contacts. ";
        narrative += if pirate_count == 1 {
            "The raider breaks off immediately, fleeing into the void."
        } else {
            "The pirates scatter, each one burning hard for different jump points. They won't be back."
        };

        // Bonus fame for clever tactics
        create_success_result(25, 10, narrative)
    } else {
        // Failed - they see through it
        let narrative = "Your deception fails. The pirate captain laughs over an open channel: 'Nice try, Compact dog. Now you die.' They're coming for you.";
        create_combat_result(narrative)
    }
}

fn handle_negotiation(ctx, pirate_count) {
    let fame = ctx["player_fame"];

    // Success based on fame (reputation intimidation)
    let success_chance = 0.3 + (fame / 200.0);
    success_chance = clamp(success_chance, 0.2, 0.8);

    if rand() < success_chance {
        let narrative = "You broadcast on all frequencies, your reputation preceding you. ";
        narrative += if fame > 60 {
            "The pirate captain recognizes your callsign. 'Not worth it,' they mutter, and the raiders peel off without another word."
        } else {
            "After a tense moment, the lead pirate responds: 'We know you. Fine. The freighter's not worth losing ships over.' They withdraw."
        };

        // Good rep and fame for using reputation effectively
        create_success_result(30, 15, narrative)
    } else {
        let narrative = "The pirates aren't impressed. 'Pretty words from a pretty ship. Let's see how you fight.' They're coming.";
        create_combat_result(narrative)
    }
}

fn handle_abandon(ctx, pirate_count) {
    // Always "succeeds" but with costs
    let narrative = "You hold position as the pirates board the freighter. They take the cargo - ";
    narrative += "rare metals worth a small fortune - but the crew is evacuated safely to your ship. ";
    narrative += "The freighter captain thanks you through tears. You saved lives. But the cargo... the admiralty won't be pleased.";

    // Small rep gain (saved lives), no fame (not heroic), but cargo lost
    create_success_result(10, 0, narrative)
}

// Called when combat resolves
fn on_combat_resolved(ctx, combat_result) {
    if combat_result["player_won"] {
        let narrative = "The last pirate vessel breaks apart under your guns. ";

        // Check how damaged we got
        let hull = ctx["ship_hull"];
        let fame_bonus = 0;

        if hull < 30.0 {
            narrative += "Your own ship is barely holding together - hull breaches seal with emergency foam, systems spark and flicker. But you won.";
            fame_bonus = 15; // Heroic survival
        } else if hull < 60.0 {
            narrative += "Your ship bears the scars of battle, but nothing that can't be repaired.";
            fame_bonus = 5;
        } else {
            narrative += "A clean victory. The pirates never stood a chance.";
        }

        narrative += " The freighter captain floods your comms with gratitude.";

        create_success_result(35 + fame_bonus, 20 + fame_bonus, narrative)
    } else {
        let narrative = "Your ship shudders as critical systems fail. Emergency protocols engage, pulling you out of the fight. ";
        narrative += "Through damaged sensors, you watch the pirates finish stripping the freighter. You failed.";

        create_failure_result(25, narrative)
    }
}
