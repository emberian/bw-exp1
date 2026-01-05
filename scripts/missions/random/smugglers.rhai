// Smugglers Mission Script
// A suspicious vessel - legitimate trader, minor smuggler, or major criminal?
// Requires judgment calls and careful investigation.
//
// States: start -> intercept -> inspection -> resolution

// Called when mission starts
fn on_start(ctx) {
    // Determine what the ship actually is
    let roll = rand();
    let vessel_type = if roll < 0.35 {
        "legitimate"    // Clean trader, you're wasting time
    } else if roll < 0.65 {
        "minor"         // Small-time smuggler
    } else if roll < 0.85 {
        "major"         // Serious criminal operation
    } else {
        "hollow"        // Hollow Circuit operative (special case)
    };

    // Ship characteristics
    let ship_names = ["Stellar Dream", "Void Runner", "Lucky Strike", "Meridian's Hope", "Silent Passage"];
    let ship_name = ship_names[rand_int(0, 4)];

    let narrative = "Patrol sensors flag a cargo vessel on an unusual trajectory. ";
    narrative += "The " + ship_name + " is running without a filed flight plan, ";
    narrative += "skirting the edge of the shipping lanes. Could be nothing. Could be everything.";

    let choices = [];

    choices.push(create_choice("intercept", "Move to intercept and demand identification"));

    if meets_requirement(ctx, "MinExperience", 40) {
        choices.push(create_choice("shadow", "Shadow them quietly and observe"));
    } else {
        choices.push(create_choice_with_req("shadow", "Shadow and observe", "MinExperience", 40));
    }

    choices.push(create_choice("scan", "Perform a long-range cargo scan"));

    choices.push(create_choice("ignore", "Log and continue - they're probably legitimate"));

    let data = #{
        vessel_type: vessel_type,
        ship_name: ship_name,
        discovered: false,
        captain_cooperative: rand() < 0.6
    };

    create_choice_result("patrol", narrative, choices)
}

fn on_choice(ctx, choice_id) {
    let data = get_mission_data(ctx);
    let vessel_type = data["vessel_type"];
    let ship_name = data["ship_name"];
    let cooperative = data["captain_cooperative"];

    if choice_id == "intercept" {
        handle_intercept(ctx, vessel_type, ship_name, cooperative, data)
    } else if choice_id == "shadow" {
        handle_shadow(ctx, vessel_type, ship_name, data)
    } else if choice_id == "scan" {
        handle_scan(ctx, vessel_type, ship_name, data)
    } else if choice_id == "ignore" {
        handle_ignore(ctx, vessel_type)
    } else if choice_id == "board" {
        handle_board(ctx, vessel_type, ship_name, data)
    } else if choice_id == "let_go" {
        handle_let_go(ctx, vessel_type, ship_name)
    } else if choice_id == "arrest" {
        handle_arrest(ctx, vessel_type, ship_name)
    } else if choice_id == "negotiate" {
        handle_negotiate_hollow(ctx, ship_name)
    } else if choice_id == "confiscate" {
        handle_confiscate(ctx, vessel_type, ship_name)
    } else if choice_id == "warn" {
        handle_warn(ctx, vessel_type, ship_name)
    } else if choice_id == "pursue" {
        handle_pursue(ctx, vessel_type, ship_name)
    } else {
        create_failure_result(5, "Unknown action taken.")
    }
}

fn handle_intercept(ctx, vessel_type, ship_name, cooperative, data) {
    let narrative = "You broadcast authority codes and order the " + ship_name + " to heave to for inspection. ";

    if vessel_type == "legitimate" {
        if cooperative {
            narrative += "The captain complies immediately, transmitting credentials. 'Of course, officer. Happy to assist.'";
        } else {
            narrative += "The captain complies, but their tone is cold. 'This is harassment. I have rights.'";
        }

        let choices = [];
        choices.push(create_choice("board", "Board for physical inspection"));
        choices.push(create_choice("let_go", "Credentials check out - let them go"));

        create_choice_result("inspection", narrative, choices)

    } else if vessel_type == "minor" {
        if cooperative {
            narrative += "A nervous face appears on screen. 'O-of course, officer. Everything's in order. Mostly.'";
            narrative += " They're sweating.";
        } else {
            narrative += "The captain's eyes dart around. 'What? Why? I'm a legitimate businessman.' ";
            narrative += "They're definitely hiding something.";
        }

        let choices = [];
        choices.push(create_choice("board", "Board for physical inspection"));
        choices.push(create_choice("let_go", "Let them proceed with a warning"));

        if meets_requirement(ctx, "MinFame", 20) {
            choices.push(create_choice("intimidate", "Use your reputation to pressure a confession"));
        }

        create_choice_result("inspection", narrative, choices)

    } else if vessel_type == "major" {
        if cooperative {
            narrative += "The captain is calm - too calm. 'Of course. Come aboard.' Something feels wrong.";
        } else {
            narrative += "The response is silence. Then: 'Space Guard vessel, I recommend you find elsewhere to patrol.' ";
            narrative += "The " + ship_name + "'s weapons are powering up.";
        }

        let choices = [];
        if cooperative {
            choices.push(create_choice("board", "Board - carefully"));
            choices.push(create_choice("call_backup", "Request backup before boarding"));
        } else {
            choices.push(create_choice("pursue", "Pursue and disable"));
            choices.push(create_choice("let_go", "Not worth the fight - let them go"));
        }

        create_choice_result("confrontation", narrative, choices)

    } else {
        // Hollow Circuit
        narrative += "The captain smiles knowingly. 'Ah, Space Guard. I was wondering when we'd meet.' ";
        narrative += "They're not afraid. They're... amused? 'I have information that might interest you.'";

        let choices = [];
        choices.push(create_choice("board", "Board and inspect regardless"));
        choices.push(create_choice("negotiate", "Hear what they have to say"));
        choices.push(create_choice("arrest", "Arrest them immediately"));

        create_choice_result("hollow_contact", narrative, choices)
    }
}

fn handle_shadow(ctx, vessel_type, ship_name, data) {
    let experience = get_crew_experience(ctx);
    let stealth_success = 0.4 + experience_success_chance(0.3, experience);

    if rand() < stealth_success {
        // Successful shadow
        if vessel_type == "legitimate" {
            let narrative = "You follow the " + ship_name + " for two hours. They dock at a mining station, ";
            narrative += "conduct legitimate business, and depart. You've wasted time on a clean trader.";

            create_success_result(0, 0, narrative)

        } else if vessel_type == "minor" {
            let narrative = "You observe the " + ship_name + " rendezvousing with a small shuttle in deep space. ";
            narrative += "Quick exchange of cargo containers. Classic dead drop. Small-time, but illegal.";

            let choices = [];
            choices.push(create_choice("intercept", "Move in now - you have evidence"));
            choices.push(create_choice("track_shuttle", "Let them go and track the shuttle instead"));

            create_choice_result("observed_handoff", narrative, choices)

        } else if vessel_type == "major" {
            let narrative = "The " + ship_name + " leads you to a hidden outpost in an asteroid field. ";
            narrative += "Multiple vessels. Heavy weapons. This is a major smuggling operation. ";
            narrative += "Too big for you alone, but intelligence like this is invaluable.";

            let choices = [];
            choices.push(create_choice("report_base", "Mark location and report to command"));
            choices.push(create_choice("solo_strike", "Attack while you have surprise"));

            create_choice_result("discovered_base", narrative, choices)

        } else {
            // Hollow Circuit
            let narrative = "The " + ship_name + " stops in empty space. Waits. ";
            narrative += "A second ship decloaks - no transponder, no markings. ";
            narrative += "They exchange... something. Data? The unknown ship vanishes. ";
            narrative += "The " + ship_name + "'s captain waves at your hiding spot. They knew you were there.";

            let choices = [];
            choices.push(create_choice("intercept", "Intercept now"));
            choices.push(create_choice("retreat", "Something's very wrong here"));

            create_choice_result("hollow_mystery", narrative, choices)
        }
    } else {
        // Spotted
        let narrative = "Your attempt at stealth fails - the " + ship_name + " spots your approach. ";

        if vessel_type == "major" || vessel_type == "hollow" {
            narrative += "They immediately jump to FTL. Gone. You've spooked them.";
            create_failure_result(5, narrative)
        } else {
            narrative += "They change course but don't run. You can still intercept.";

            let choices = [];
            choices.push(create_choice("intercept", "Intercept now"));

            create_choice_result("spotted", narrative, choices)
        }
    }
}

fn handle_scan(ctx, vessel_type, ship_name, data) {
    if vessel_type == "legitimate" {
        let narrative = "Long-range scans show standard cargo: processed materials, medical supplies, consumer goods. ";
        narrative += "Nothing suspicious. The " + ship_name + " appears to be a legitimate trader.";

        let choices = [];
        choices.push(create_choice("intercept", "Intercept anyway to be thorough"));
        choices.push(create_choice("ignore", "Let them proceed"));

        create_choice_result("clean_scan", narrative, choices)

    } else if vessel_type == "minor" {
        let narrative = "Your scans detect anomalies - shielded containers that resist scanning. ";
        narrative += "Could be expensive goods with anti-theft protection. Could be contraband.";

        let choices = [];
        choices.push(create_choice("intercept", "Intercept for physical inspection"));

        create_choice_result("suspicious_scan", narrative, choices)

    } else if vessel_type == "major" {
        let narrative = "Heavy sensor shielding across the entire cargo bay. Military-grade countermeasures. ";
        narrative += "Whatever they're carrying, they've invested serious resources in hiding it. ";
        narrative += "This is no small-time operation.";

        let choices = [];
        choices.push(create_choice("intercept", "Intercept immediately"));
        choices.push(create_choice("call_backup", "Request backup - this looks big"));

        create_choice_result("heavy_shielding", narrative, choices)

    } else {
        // Hollow Circuit - scans show NOTHING
        let narrative = "Your sensors return... nothing. The ship registers as empty. ";
        narrative += "Not shielded. Empty. But you can see the vessel right there. ";
        narrative += "Their tech is beyond anything you've encountered.";

        let choices = [];
        choices.push(create_choice("intercept", "Intercept and demand answers"));
        choices.push(create_choice("retreat", "Report this to command and withdraw"));

        create_choice_result("impossible_scan", narrative, choices)
    }
}

fn handle_ignore(ctx, vessel_type) {
    if vessel_type == "legitimate" {
        let narrative = "Good judgment. The vessel proceeds without incident.";
        create_success_result(0, 0, narrative)
    } else if vessel_type == "minor" {
        let narrative = "The vessel continues on its way. Somewhere, contraband changes hands. ";
        narrative += "You'll never know what you missed.";
        create_success_result(0, 0, narrative)
    } else {
        let narrative = "The vessel vanishes into the void. Later, you hear about a major shipment ";
        narrative += "of illegal weapons reaching terrorist hands. You wonder.";
        create_failure_result(0, narrative)
    }
}

fn handle_board(ctx, vessel_type, ship_name, data) {
    if vessel_type == "legitimate" {
        let narrative = "A thorough search reveals nothing illegal. The captain's irritation is palpable. ";
        narrative += "'Satisfied, officer? I'll be filing a complaint.' ";
        narrative += "You've wasted time and made an enemy.";

        create_failure_result(5, narrative)

    } else if vessel_type == "minor" {
        let narrative = "Hidden in a false panel: recreational pharmaceuticals, untaxed luxury goods. ";
        narrative += "The captain's face falls. 'Look, I'm just trying to make a living. ";
        narrative += "The taxes on these goods are impossible. Please...'";

        let choices = [];
        choices.push(create_choice("arrest", "Arrest and impound the cargo"));
        choices.push(create_choice("confiscate", "Confiscate the contraband but let them go"));
        choices.push(create_choice("warn", "Issue a stern warning"));

        create_choice_result("found_contraband", narrative, choices)

    } else if vessel_type == "major" {
        let narrative = "The moment you board, you know something's wrong. The crew is too disciplined. ";
        narrative += "Too armed. Your sensors couldn't see it, but now you can: weapons. ";
        narrative += "Military grade. Enough to arm a small army. This is serious.";

        // Risk of combat
        if rand() < 0.4 {
            narrative += " They make their move - weapons drawn.";
            create_combat_result(narrative)
        } else {
            narrative += " But they're outnumbered and outgunned. They surrender.";

            let choices = [];
            choices.push(create_choice("arrest", "Arrest the crew and impound everything"));
            choices.push(create_choice("interrogate", "Interrogate them about their buyers"));

            create_choice_result("major_bust", narrative, choices)
        }

    } else {
        // Hollow Circuit
        let narrative = "The ship is... ordinary. Impossibly so. You find nothing unusual. ";
        narrative += "But the captain watches you with amusement. 'Find what you were looking for?' ";
        narrative += "You haven't. But you know they're hiding something. Somehow.";

        create_success_result(10, 0, narrative)
    }
}

fn handle_arrest(ctx, vessel_type, ship_name) {
    if vessel_type == "minor" {
        let narrative = "You arrest the captain and crew, impounding the " + ship_name + " and its cargo. ";
        narrative += "A small victory against the black market. The paperwork takes hours.";
        create_success_result(20, 5, narrative)
    } else if vessel_type == "major" {
        let narrative = "Major arrest. The weapons alone are worth a fortune. ";
        narrative += "Intelligence is already interested in who was supposed to receive this shipment. ";
        narrative += "This will make waves in the right places.";
        create_success_result(45, 25, narrative)
    } else {
        // Hollow Circuit - bad idea
        let narrative = "You attempt to arrest the Hollow Circuit operative. ";
        narrative += "By the time backup arrives, they've vanished. The ship is empty. ";
        narrative += "You find a note on your console: 'We'll be in touch. - HC'";
        create_success_result(5, 10, narrative)
    }
}

fn handle_negotiate_hollow(ctx, ship_name) {
    let narrative = "The Hollow Circuit operative smiles. 'Smart choice.' ";
    narrative += "They transfer a data packet to your secure terminal. ";
    narrative += "'The location of a Sera scout. It's been tracking shipping lanes. ";
    narrative += "We thought you should know.' ";
    narrative += "Before you can respond, the " + ship_name + " jumps away.";

    // Valuable intelligence, questionable source
    create_success_result(15, 10, narrative)
}

fn handle_confiscate(ctx, vessel_type, ship_name) {
    let narrative = "You confiscate the contraband and issue a formal warning. ";
    narrative += "The captain is grateful not to lose their ship. ";
    narrative += "'I owe you one, officer. I won't forget this.'";

    // Moderate rep, future contact maybe
    create_success_result(15, 0, narrative)
}

fn handle_warn(ctx, vessel_type, ship_name) {
    let narrative = "You let them go with a warning. The captain's relief is visible. ";
    narrative += "'Thank you. I swear, I'll go legitimate.' ";
    narrative += "You've shown mercy. Whether that's wisdom or weakness remains to be seen.";

    create_success_result(5, 0, narrative)
}

fn handle_pursue(ctx, vessel_type, ship_name) {
    let narrative = "You give chase. The " + ship_name + " is fast, but you're faster. ";
    narrative += "They turn to fight rather than be caught.";

    create_combat_result(narrative)
}

fn on_combat_resolved(ctx, combat_result) {
    if combat_result["player_won"] {
        let narrative = "The smugglers are disabled. Survivors are taken into custody. ";
        narrative += "Their cargo reveals the scope of the operation - this will hurt the syndicates badly.";
        create_success_result(40, 20, narrative)
    } else {
        let narrative = "They got away. Your ship limps back to port. ";
        narrative += "At least you have their transponder data.";
        create_failure_result(10, narrative)
    }
}
