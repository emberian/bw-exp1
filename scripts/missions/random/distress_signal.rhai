// Distress Signal Mission Script
// An automated distress beacon - could be genuine, a trap, or something stranger.
//
// States: start -> investigation -> (combat|rescue|discovery) -> complete

// Called when mission starts
fn on_start(ctx) {
    // Determine what the signal actually is
    let roll = rand();
    let signal_type = if roll < 0.4 {
        "genuine"      // Real emergency
    } else if roll < 0.7 {
        "trap"         // Pirate ambush
    } else if roll < 0.9 {
        "derelict"     // Abandoned ship
    } else {
        "anomaly"      // Something strange
    };

    let narrative = "Your sensors detect an automated distress beacon. ";
    narrative += if signal_type == "genuine" {
        "The signal is weak but consistent - standard civilian emergency protocol. Someone needs help."
    } else if signal_type == "trap" {
        "The signal pattern is... slightly off. Your crew can't quite place it, but something feels wrong."
    } else if signal_type == "derelict" {
        "Cross-referencing the beacon ID shows the vessel was reported missing three cycles ago. No life signs detected."
    } else {
        "The signal doesn't match any known protocol. It seems to pulse in patterns that almost form... words?"
    };

    let choices = [];

    // Investigate closely
    choices.push(create_choice("investigate", "Move in to investigate the source"));

    // Scan first (experience-gated)
    if meets_requirement(ctx, "MinExperience", 30) {
        choices.push(create_choice("scan", "Perform a detailed sensor sweep first"));
    } else {
        choices.push(create_choice_with_req("scan", "Perform detailed scan", "MinExperience", 30));
    }

    // Broadcast response
    choices.push(create_choice("hail", "Broadcast on emergency frequencies"));

    // Ignore (always available, cowardly)
    choices.push(create_choice("ignore", "Log the coordinates and continue patrol"));

    let data = #{
        signal_type: signal_type,
        scanned: false,
        trap_revealed: false
    };

    create_choice_result("encounter", narrative, choices)
}

fn on_choice(ctx, choice_id) {
    let data = get_mission_data(ctx);
    let signal_type = data["signal_type"];

    // Initial choices
    if choice_id == "investigate" {
        handle_investigate(ctx, signal_type, data)
    } else if choice_id == "scan" {
        handle_scan(ctx, signal_type, data)
    } else if choice_id == "hail" {
        handle_hail(ctx, signal_type, data)
    } else if choice_id == "ignore" {
        handle_ignore(ctx, signal_type)
    }
    // Rescue choices
    else if choice_id == "rescue_full" {
        handle_rescue_full(ctx)
    } else if choice_id == "rescue_evac" {
        handle_rescue_evac(ctx)
    } else if choice_id == "rescue_tow" {
        handle_rescue_tow(ctx)
    }
    // Trap choices
    else if choice_id == "ambush_them" {
        handle_ambush(ctx)
    } else if choice_id == "report_trap" {
        handle_report_trap(ctx)
    }
    // Derelict choices
    else if choice_id == "salvage" {
        handle_salvage(ctx)
    } else if choice_id == "investigate_derelict" {
        handle_investigate_derelict(ctx)
    } else if choice_id == "report" {
        handle_report_derelict(ctx)
    }
    // Anomaly choices
    else if choice_id == "probe" {
        handle_probe_anomaly(ctx)
    } else if choice_id == "enter" {
        handle_enter_anomaly(ctx)
    }
    // General
    else if choice_id == "retreat" || choice_id == "withdraw" {
        handle_withdraw(ctx)
    }
    else {
        create_failure_result(5, "Mission parameters corrupted.")
    }
}

fn handle_investigate(ctx, signal_type, data) {
    if signal_type == "genuine" {
        // Real emergency - damaged ship
        let narrative = "You approach to find a civilian transport, engines dark, life support failing. ";
        narrative += "The captain's face appears on screen, relief flooding their features. ";
        narrative += "'Thank the makers. Our reactor destabilized - we've got maybe an hour of air left.'";

        let choices = [];
        choices.push(create_choice("rescue_full", "Transfer crew and attempt to stabilize their reactor"));
        choices.push(create_choice("rescue_evac", "Evacuate survivors immediately"));

        if meets_requirement(ctx, "MinFuel", 30.0) {
            choices.push(create_choice("rescue_tow", "Transfer fuel and escort them to port"));
        }

        create_choice_result("rescue", narrative, choices)

    } else if signal_type == "trap" && !data["trap_revealed"] {
        // Ambush!
        let narrative = "As you close the distance, your sensors scream a warning. ";
        narrative += "The 'distress beacon' is attached to an asteroid - bait. ";
        narrative += "Two pirate raiders decloak behind you, weapons hot.";

        create_combat_result(narrative)

    } else if signal_type == "trap" && data["trap_revealed"] {
        // Trap was revealed by scanning
        let narrative = "You approach cautiously, knowing the trap. As expected, two raiders decloak - ";
        narrative += "but you've already powered weapons and shields. ";
        narrative += "Their surprise attack becomes a fair fight.";

        // Combat with advantage
        create_combat_result(narrative)

    } else if signal_type == "derelict" {
        let narrative = "The ship drifts dead in space, hull breached in multiple locations. ";
        narrative += "Your scans show no life signs, but the cargo hold appears intact. ";
        narrative += "Whatever happened here, it happened fast.";

        let choices = [];
        choices.push(create_choice("salvage", "Board and salvage what you can"));
        choices.push(create_choice("investigate_derelict", "Search for clues about what happened"));
        choices.push(create_choice("report", "Mark the location and report to authorities"));

        create_choice_result("derelict", narrative, choices)

    } else {
        // Anomaly
        let narrative = "As you approach, your instruments begin to fluctuate. ";
        narrative += "The distress signal isn't coming from a ship at all - it's emanating from a ";
        narrative += "shifting pattern of energy that hurts to look at directly. ";
        narrative += "Your crew reports headaches. One swears they heard singing.";

        let choices = [];
        choices.push(create_choice("retreat", "Back away slowly and report this"));
        choices.push(create_choice("probe", "Launch a sensor probe into the anomaly"));

        if meets_requirement(ctx, "MinExperience", 100) {
            choices.push(create_choice("enter", "Take the ship closer for direct observation"));
        }

        create_choice_result("anomaly", narrative, choices)
    }
}

fn handle_scan(ctx, signal_type, data) {
    let experience = get_crew_experience(ctx);
    let scan_quality = 0.5 + (experience / 200.0);

    if signal_type == "trap" && rand() < scan_quality {
        // Detected the trap!
        let narrative = "Your sensor sweep reveals hidden power signatures - ships running silent. ";
        narrative += "This 'distress signal' is a trap. At least two raiders are waiting to ambush anyone who investigates.";

        let choices = [];
        choices.push(create_choice("ambush_them", "Turn the tables - attack before they know you've spotted them"));
        choices.push(create_choice("report_trap", "Report the pirate position to sector command"));
        choices.push(create_choice("withdraw", "Withdraw before they realize you've spotted them"));

        // Mark trap as revealed
        let new_data = #{
            signal_type: signal_type,
            scanned: true,
            trap_revealed: true
        };

        create_choice_result_with_data("trap_revealed", narrative, choices, new_data)

    } else if signal_type == "genuine" {
        let narrative = "Scans confirm: civilian vessel, catastrophic reactor failure. ";
        narrative += "Life signs are weak but present. They're running out of time.";

        let choices = [];
        choices.push(create_choice("investigate", "Move in immediately to help"));

        create_choice_result("scanned_genuine", narrative, choices)

    } else if signal_type == "derelict" {
        let narrative = "Detailed scans reveal no life signs and significant hull damage consistent with weapons fire. ";
        narrative += "This ship was attacked, not accident-damaged. The pirates may still be nearby.";

        let choices = [];
        choices.push(create_choice("investigate", "Proceed with caution"));
        choices.push(create_choice("withdraw", "Too risky - withdraw and report"));

        create_choice_result("scanned_derelict", narrative, choices)

    } else {
        // Anomaly - scans don't help much
        let narrative = "Your sensors return... impossible data. The readings contradict each other. ";
        narrative += "Whatever this is, it's not following standard physics.";

        let choices = [];
        choices.push(create_choice("investigate", "Investigate directly"));
        choices.push(create_choice("retreat", "This is above your pay grade"));

        create_choice_result("scanned_anomaly", narrative, choices)
    }
}

fn handle_hail(ctx, signal_type, data) {
    if signal_type == "genuine" {
        let narrative = "'Oh thank the void!' A ragged voice responds. 'Our reactor is failing - we don't have long. Please, we have children aboard.'";

        let choices = [];
        choices.push(create_choice("investigate", "We're on our way. Hold tight."));

        create_choice_result("hail_response", narrative, choices)

    } else if signal_type == "trap" {
        // Pirates respond with fake distress
        let narrative = "'Please... help us...' The voice is stilted, unconvincing. Your comm officer raises an eyebrow.";

        let choices = [];
        choices.push(create_choice("investigate", "Proceed anyway"));
        choices.push(create_choice("scan", "Something's wrong - scan first"));

        create_choice_result("suspicious_hail", narrative, choices)

    } else if signal_type == "derelict" {
        let narrative = "Static. No response. The silence is somehow worse than any answer.";

        let choices = [];
        choices.push(create_choice("investigate", "Move in to investigate"));
        choices.push(create_choice("ignore", "They're beyond help"));

        create_choice_result("no_response", narrative, choices)

    } else {
        // Anomaly responds... somehow
        let narrative = "The signal changes. Numbers begin scrolling across your screen - coordinates, ";
        narrative += "dates, what might be names. Then, clearly: YOUR ship's designation. ";
        narrative += "Whatever this is, it knows you're here.";

        let choices = [];
        choices.push(create_choice("investigate", "...approach it"));
        choices.push(create_choice("retreat", "Get out of here. Now."));

        create_choice_result("anomaly_response", narrative, choices)
    }
}

fn handle_ignore(ctx, signal_type) {
    let narrative = "You log the distress beacon coordinates and continue your patrol. ";

    if signal_type == "genuine" {
        narrative += "The signal grows weaker behind you. You try not to think about it.";
        // Small rep loss for ignoring genuine distress
        create_failure_result(5, narrative)
    } else if signal_type == "trap" {
        narrative += "Some instinct tells you that was the right call.";
        // No penalty - avoided trap
        create_success_result(5, 0, narrative)
    } else {
        narrative += "Whatever was out there, it's not your problem anymore.";
        create_success_result(0, 0, narrative)
    }
}

fn handle_rescue_full(ctx) {
    let experience = get_crew_experience(ctx);
    let success_chance = 0.5 + experience_success_chance(0.2, experience);

    if rand() < success_chance {
        let narrative = "Your engineering crew works miracles. The reactor stabilizes just as the last evacuation shuttle launches. ";
        narrative += "The transport captain weeps with relief - their ship can limp to port. ";
        narrative += "You've saved lives AND their livelihood.";

        // Big rewards
        create_success_result(40, 15, narrative)
    } else {
        let narrative = "Despite your crew's best efforts, the reactor can't be saved. ";
        narrative += "You evacuate everyone moments before the ship tears itself apart. ";
        narrative += "Lives saved, but their ship is gone.";

        create_success_result(25, 5, narrative)
    }
}

fn handle_rescue_evac(ctx) {
    let narrative = "You prioritize lives over machinery. The evacuation is swift and orderly. ";
    narrative += "Thirty-two souls transfer to your ship before the transport finally dies. ";
    narrative += "The captain's gratitude is tinged with grief - that was their whole life.";

    create_success_result(25, 5, narrative)
}

fn handle_rescue_tow(ctx) {
    let narrative = "You transfer emergency fuel reserves and take the crippled transport under tow. ";
    narrative += "It's slow going, but three hours later you deliver them safely to Thornwick Station. ";
    narrative += "The dockmaster buys your crew drinks for a week.";

    // Costs fuel but big reputation
    create_success_with_cost_result(35, 10, -15.0, 0.0, narrative)
}

fn handle_ambush(ctx) {
    let narrative = "You spin your ship and light up the darkness with targeting lasers. ";
    narrative += "The pirates scramble - they weren't ready to be the prey. ";
    narrative += "You have the advantage of surprise.";

    // Combat with advantage (would need special handling)
    create_combat_result(narrative)
}

fn handle_report_trap(ctx) {
    let narrative = "You transmit the pirate positions to sector command and withdraw. ";
    narrative += "Within the hour, a Flotilla patrol sweeps the area. The pirates scatter, but at least two are captured. ";
    narrative += "Command sends commendations for the intelligence work.";

    create_success_result(30, 5, narrative)
}

fn handle_salvage(ctx) {
    let roll = rand();
    if roll < 0.7 {
        let narrative = "The salvage operation goes smoothly. Medical supplies, processed materials, ";
        narrative += "and - surprisingly - a cache of pre-Cataclysm artifacts in the cargo hold. ";
        narrative += "The Remnant will pay well for these.";

        create_success_result(15, 10, narrative)
    } else {
        let narrative = "As your crew boards, residual systems come online. ";
        narrative += "Automated defenses activate - the ship wasn't as dead as you thought. ";
        narrative += "You're forced to fight your way back to the airlock.";

        create_success_with_cost_result(10, 5, 0.0, -15.0, narrative)
    }
}

fn handle_investigate_derelict(ctx) {
    let narrative = "In the captain's quarters, you find the final log entry. ";
    narrative += "Pirates hit them three days ago. They tried to run but the engines failed. ";
    narrative += "The captain's last words were for someone named Elena. ";
    narrative += "You save the logs - maybe someone will want to know what happened.";

    create_success_result(10, 0, narrative)
}

fn handle_report_derelict(ctx) {
    let narrative = "You flag the location for recovery teams and continue your patrol. ";
    narrative += "Proper procedure. Not heroic, but necessary work.";

    create_success_result(10, 0, narrative)
}

fn handle_probe_anomaly(ctx) {
    let narrative = "The probe enters the anomaly and... stops transmitting coherent data. ";
    narrative += "Then, impossibly, it emerges from the anomaly BEHIND your ship. ";
    narrative += "The timestamp on its last recording is three days in the future. ";
    narrative += "Your crew is very quiet on the return trip.";

    // Strange experience, good for fame
    create_success_result(20, 25, narrative)
}

fn handle_enter_anomaly(ctx) {
    let narrative = "You take your ship into the impossible light. ";
    narrative += "Time... skips. You emerge with no memory of what happened inside. ";
    narrative += "Your chronometer shows four hours passed, but your crew insists it was seconds. ";
    narrative += "The anomaly is gone. Your sensors recorded... music?";

    // Big fame, unsettling
    create_success_result(30, 40, narrative)
}

fn handle_withdraw(ctx) {
    let narrative = "You plot a course away from here. Some mysteries aren't worth pursuing.";
    create_success_result(0, 0, narrative)
}

fn on_combat_resolved(ctx, combat_result) {
    let data = get_mission_data(ctx);

    if combat_result["player_won"] {
        let narrative = "The pirates break and run. ";
        if data["trap_revealed"] {
            narrative += "They never expected you to be ready for them. Their trap became their coffin.";
            // Bonus for turning tables
            create_success_result(35, 20, narrative)
        } else {
            narrative += "You survived their ambush - barely. They won't catch anyone else with that trick.";
            create_success_result(25, 15, narrative)
        }
    } else {
        let narrative = "You're forced to retreat, systems failing. The pirates let you go - probably not worth the ammunition.";
        create_failure_result(15, narrative)
    }
}
