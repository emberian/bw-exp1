// Terrorist Plot Mission Script
// Intelligence indicates a planned attack. Investigation, infiltration, and hard choices.
//
// States: start -> investigation -> (infiltrate|assault|negotiate) -> resolution

fn on_start(ctx) {
    // Determine the plot type
    let roll = rand();
    let plot_type = if roll < 0.4 {
        "bomb"          // Explosive device on station
    } else if roll < 0.7 {
        "sabotage"      // Infrastructure attack
    } else if roll < 0.9 {
        "hostage"       // Taken hostages
    } else {
        "ideological"   // Propaganda action (less violent)
    };

    // Determine the group
    let groups = ["Sera Remnants", "Machine Liberation", "Flesh Purists", "Corporate Separatists"];
    let group = groups[rand_int(0, 3)];

    let narrative = "CLASSIFIED ALERT: Intelligence indicates imminent terrorist action. ";

    if plot_type == "bomb" {
        narrative += "The " + group + " have planted an explosive device somewhere on Thornwick Station. ";
        narrative += "Unknown yield. Unknown location. Civilian population: 12,000.";
    } else if plot_type == "sabotage" {
        narrative += "The " + group + " plan to sabotage the station's life support. ";
        narrative += "Slow death for thousands unless we find their operative.";
    } else if plot_type == "hostage" {
        narrative += "The " + group + " have seized the medical bay with 47 hostages. ";
        narrative += "They're demanding the release of imprisoned members. ";
        narrative += "Deadline: 6 hours.";
    } else {
        narrative += "The " + group + " plan to broadcast a manifesto by hijacking station comms. ";
        narrative += "Non-violent, but the propaganda could inspire worse actions.";
    }

    let choices = [];

    choices.push(create_choice("investigate", "Begin investigation - find more information"));

    if plot_type == "hostage" {
        choices.push(create_choice("negotiate_start", "Open negotiations with the terrorists"));
        if meets_requirement(ctx, "MinExperience", 70) {
            choices.push(create_choice("assault", "Immediate tactical assault"));
        }
    }

    if meets_requirement(ctx, "MinFame", 40) {
        choices.push(create_choice("contacts", "Use your reputation to find informants"));
    }

    let data = #{
        plot_type: plot_type,
        group: group,
        intel_level: 0,
        time_remaining: 360,  // minutes
        hostages_alive: 47,
        bomb_found: false
    };

    create_choice_result("briefing", narrative, choices)
}

fn on_choice(ctx, choice_id) {
    let data = get_mission_data(ctx);

    if choice_id == "investigate" {
        handle_investigate(ctx, data)
    } else if choice_id == "contacts" {
        handle_contacts(ctx, data)
    } else if choice_id == "negotiate_start" {
        handle_negotiate_start(ctx, data)
    } else if choice_id == "assault" {
        handle_assault(ctx, data)
    } else if choice_id == "search" {
        handle_search(ctx, data)
    } else if choice_id == "infiltrate" {
        handle_infiltrate(ctx, data)
    } else if choice_id == "meet_demands" {
        handle_meet_demands(ctx, data)
    } else if choice_id == "stall" {
        handle_stall(ctx, data)
    } else if choice_id == "sniper" {
        handle_sniper(ctx, data)
    } else if choice_id == "disarm" {
        handle_disarm(ctx, data)
    } else if choice_id == "evacuate_sector" {
        handle_evacuate_sector(ctx, data)
    } else if choice_id == "confront" {
        handle_confront(ctx, data)
    } else if choice_id == "let_broadcast" {
        handle_let_broadcast(ctx, data)
    } else {
        create_failure_result(10, "Time wasted. The plot succeeds.")
    }
}

fn handle_investigate(ctx, data) {
    let plot_type = data["plot_type"];
    let group = data["group"];
    let experience = get_crew_experience(ctx);

    let intel_bonus = experience_success_chance(2.0, experience);
    let new_intel = data["intel_level"] + 1 + intel_bonus.to_int();

    let narrative = "Your investigation yields results. ";

    if plot_type == "bomb" {
        if new_intel >= 2 {
            narrative += "Cross-referencing cargo manifests with known explosive precursors, ";
            narrative += "you narrow the bomb location to three possible areas: ";
            narrative += "the reactor housing, the docking bay, or the market district.";

            let choices = [];
            choices.push(create_choice("search", "Search all three locations"));

            if new_intel >= 3 {
                choices.push(create_choice("disarm", "You've pinpointed it - move to disarm"));
            }

            choices.push(create_choice("evacuate_sector", "Evacuate all three areas"));

            let new_data = #{
                plot_type: data["plot_type"],
                group: data["group"],
                intel_level: new_intel,
                time_remaining: data["time_remaining"] - 60,
                hostages_alive: data["hostages_alive"],
                bomb_found: false
            };

            create_choice_result_with_data("bomb_narrowed", narrative, choices, new_data)
        } else {
            narrative += "Security footage shows a suspicious figure in maintenance coveralls. ";
            narrative += "More investigation needed.";

            let choices = [];
            choices.push(create_choice("investigate", "Continue investigation"));
            choices.push(create_choice("contacts", "Use contacts for faster intel"));

            create_choice_result("bomb_clue", narrative, choices)
        }

    } else if plot_type == "sabotage" {
        if new_intel >= 2 {
            narrative += "The saboteur is a station technician - they have legitimate access. ";
            narrative += "You've identified three suspects. One is currently on shift in life support.";

            let choices = [];
            choices.push(create_choice("confront", "Confront the on-duty suspect"));
            choices.push(create_choice("investigate", "Gather more evidence first"));

            let new_data = #{
                plot_type: data["plot_type"],
                group: data["group"],
                intel_level: new_intel,
                time_remaining: data["time_remaining"] - 45,
                hostages_alive: data["hostages_alive"],
                bomb_found: data["bomb_found"]
            };

            create_choice_result_with_data("saboteur_identified", narrative, choices, new_data)
        } else {
            narrative += "The attack will target the atmospheric processors. ";
            narrative += "But you still don't know who or exactly when.";

            let choices = [];
            choices.push(create_choice("investigate", "Continue investigation"));

            create_choice_result("sabotage_target", narrative, choices)
        }

    } else if plot_type == "hostage" {
        narrative += "The terrorists have fortified the medical bay. ";
        narrative += "Three entry points, all guarded. They're organized but nervous.";

        let choices = [];
        choices.push(create_choice("infiltrate", "Attempt to infiltrate through maintenance"));
        choices.push(create_choice("assault", "Plan tactical assault"));
        choices.push(create_choice("negotiate_start", "Continue negotiations"));

        create_choice_result("hostage_intel", narrative, choices)

    } else {
        narrative += "The " + group + " have a sympathizer in station communications. ";
        narrative += "The broadcast is scheduled for peak traffic hours.";

        let choices = [];
        choices.push(create_choice("confront", "Arrest the sympathizer"));
        choices.push(create_choice("let_broadcast", "Let the broadcast proceed and monitor"));

        create_choice_result("broadcast_intel", narrative, choices)
    }
}

fn handle_contacts(ctx, data) {
    let fame = ctx["player_fame"];
    let success = 0.4 + (fame / 100.0);

    if rand() < success {
        let narrative = "Your reputation opens doors. An informant comes forward. ";
        narrative += "'The " + data["group"] + " are desperate. Their leader is a true believer, ";
        narrative += "but the others... they might be convinced to stand down.' ";

        // Big intel boost
        let new_data = #{
            plot_type: data["plot_type"],
            group: data["group"],
            intel_level: data["intel_level"] + 3,
            time_remaining: data["time_remaining"] - 30,
            hostages_alive: data["hostages_alive"],
            bomb_found: data["bomb_found"]
        };

        let choices = [];
        choices.push(create_choice("investigate", "Use this intel to focus investigation"));

        if data["plot_type"] == "hostage" {
            choices.push(create_choice("negotiate_start", "Use this intel in negotiations"));
        }

        create_choice_result_with_data("informant", narrative, choices, new_data)
    } else {
        let narrative = "No one's talking. Either they're scared, or they're sympathetic. ";
        narrative += "You've wasted time.";

        let choices = [];
        choices.push(create_choice("investigate", "Traditional investigation"));

        create_choice_result("contacts_failed", narrative, choices)
    }
}

fn handle_negotiate_start(ctx, data) {
    let narrative = "You establish communications with the terrorist leader. ";
    narrative += "A cold voice responds: 'Space Guard. We have 47 citizens. ";
    narrative += "Release our comrades, or they die. No tricks.'";

    let choices = [];
    choices.push(create_choice("meet_demands", "Begin processing prisoner release"));
    choices.push(create_choice("stall", "Stall for time while preparing assault"));

    if meets_requirement(ctx, "MinFame", 60) {
        choices.push(create_choice("persuade", "Attempt to talk them down"));
    }

    create_choice_result("negotiation", narrative, choices)
}

fn handle_assault(ctx, data) {
    let experience = get_crew_experience(ctx);
    let intel = data["intel_level"];

    // Higher intel = better assault outcomes
    let intel_float = intel + 0.0; // Convert to float
    let success_base = 0.3 + (intel_float * 0.1);
    let success = success_base + experience_success_chance(0.2, experience);

    if rand() < success {
        let casualties = rand_int(0, 3);
        let narrative = "The assault is swift and precise. Flash grenades, breaching charges, ";
        narrative += "your team moves through the terrorists before they can react. ";

        if casualties == 0 {
            narrative += "All hostages saved. All terrorists neutralized. Perfect operation.";
            create_success_result(50, 30, narrative)
        } else {
            narrative += casualties + " hostages caught in crossfire. The rest are saved.";
            create_success_result(35, 20, narrative)
        }
    } else {
        let casualties = rand_int(5, 15);
        let narrative = "The assault goes wrong. The terrorists were ready. ";
        narrative += "Gunfire in a medical bay full of civilians. ";
        narrative += casualties + " dead before you secure the room. A disaster.";

        create_failure_result(30, narrative)
    }
}

fn handle_infiltrate(ctx, data) {
    let experience = get_crew_experience(ctx);
    let success = 0.4 + experience_success_chance(0.3, experience);

    if rand() < success {
        let narrative = "You slip through maintenance tunnels, emerging behind enemy lines. ";
        narrative += "From here, you can take out the leader silently.";

        let choices = [];
        choices.push(create_choice("sniper", "Take the shot"));
        choices.push(create_choice("assault", "Signal for assault now that you're in position"));

        create_choice_result("infiltrated", narrative, choices)
    } else {
        let narrative = "You're spotted. A brief firefight in the vents. ";
        narrative += "You take down two terrorists but your position is compromised.";

        // Combat with advantage due to partial success
        create_combat_result(narrative)
    }
}

fn handle_sniper(ctx, data) {
    let experience = get_crew_experience(ctx);
    let success = 0.5 + experience_success_chance(0.3, experience);

    if rand() < success {
        let narrative = "One shot. The leader drops. The remaining terrorists panic. ";
        narrative += "Your team moves in during the confusion. ";
        narrative += "All hostages recovered. Minimal casualties.";

        create_success_result(45, 25, narrative)
    } else {
        let narrative = "You miss. The leader screams orders. ";
        narrative += "Everything happens at once.";

        create_combat_result(narrative)
    }
}

fn handle_meet_demands(ctx, data) {
    let narrative = "You coordinate the prisoner release. It takes hours, but the terrorists ";
    narrative += "keep their word - the hostages are released unharmed. ";
    narrative += "You've saved lives. But you've also freed dangerous criminals, ";
    narrative += "and shown that terrorism works.";

    // Lives saved but rep cost
    create_success_result(-10, 0, narrative)
}

fn handle_stall(ctx, data) {
    let success = 0.5;

    if rand() < success {
        let narrative = "Your stalling buys crucial time. The assault team gets into position.";

        let choices = [];
        choices.push(create_choice("assault", "Execute assault now"));

        // Better intel for assault
        let new_data = #{
            plot_type: data["plot_type"],
            group: data["group"],
            intel_level: data["intel_level"] + 2,
            time_remaining: data["time_remaining"],
            hostages_alive: data["hostages_alive"],
            bomb_found: data["bomb_found"]
        };

        create_choice_result_with_data("stalled", narrative, choices, new_data)
    } else {
        let narrative = "They see through your stalling. 'Time's up, Space Guard.' ";
        narrative += "A gunshot over the comms. One hostage dead.";

        // Lost a hostage
        let new_data = #{
            plot_type: data["plot_type"],
            group: data["group"],
            intel_level: data["intel_level"],
            time_remaining: data["time_remaining"] - 30,
            hostages_alive: data["hostages_alive"] - 1,
            bomb_found: data["bomb_found"]
        };

        let choices = [];
        choices.push(create_choice("assault", "Assault now!"));
        choices.push(create_choice("meet_demands", "Give them what they want"));

        create_choice_result_with_data("hostage_killed", narrative, choices, new_data)
    }
}

fn handle_search(ctx, data) {
    let experience = get_crew_experience(ctx);
    let intel_float = data["intel_level"] + 0.0; // Convert to float
    let success = 0.4 + (intel_float * 0.15) + experience_success_chance(0.15, experience);

    if rand() < success {
        let narrative = "You find it. A modified cargo container in the reactor housing. ";
        narrative += "Enough explosives to crack the station in half. ";
        narrative += "Timer shows 45 minutes.";

        let choices = [];
        choices.push(create_choice("disarm", "Attempt to disarm"));
        choices.push(create_choice("evacuate_sector", "Evacuate and jettison the container"));

        create_choice_result("bomb_found", narrative, choices)
    } else {
        let narrative = "You search frantically but find nothing. ";
        narrative += "The clock is ticking.";

        let choices = [];
        choices.push(create_choice("investigate", "More investigation"));
        choices.push(create_choice("evacuate_sector", "Evacuate the station"));

        create_choice_result("search_failed", narrative, choices)
    }
}

fn handle_disarm(ctx, data) {
    let experience = get_crew_experience(ctx);
    let success = 0.3 + experience_success_chance(0.4, experience);

    if rand() < success {
        let narrative = "Your hands are steady as you work. ";
        narrative += "The timer stops with 12 minutes remaining. ";
        narrative += "You exhale for the first time in an hour. ";
        narrative += "Twelve thousand people will never know how close they came.";

        create_success_result(50, 30, narrative)
    } else {
        let narrative = "The wrong wire. The timer accelerates. ";
        narrative += "You have seconds to jettison the container. ";

        if rand() < 0.5 {
            narrative += "It detonates harmlessly in space. Close. Too close.";
            create_success_result(25, 15, narrative)
        } else {
            narrative += "The blast catches the docking bay. Casualties: 47. ";
            narrative += "It could have been worse. Small comfort.";
            create_failure_result(20, narrative)
        }
    }
}

fn handle_evacuate_sector(ctx, data) {
    let narrative = "You order evacuation of the endangered sectors. ";
    narrative += "Chaos, but organized chaos. When the bomb detonates, ";
    narrative += "the damaged sections are clear. Property damage only.";

    create_success_result(30, 10, narrative)
}

fn handle_confront(ctx, data) {
    let plot_type = data["plot_type"];

    if plot_type == "sabotage" {
        let narrative = "You confront the suspect in life support. ";
        narrative += "Their eyes widen - then they reach for a weapon.";

        if rand() < 0.6 {
            narrative += " You're faster. They're in custody, the plot foiled.";
            create_success_result(35, 15, narrative)
        } else {
            narrative += " They trigger a deadman switch. Alarms blare. ";
            narrative += "You have minutes to override the sabotage.";

            // Skill check to fix
            let experience = get_crew_experience(ctx);
            if rand() < 0.4 + experience_success_chance(0.3, experience) {
                narrative += " Your crew manages to reverse the damage. Lives saved.";
                create_success_result(30, 20, narrative)
            } else {
                narrative += " Atmospheric processors fail in three sectors. ";
                narrative += "Emergency systems engage but casualties mount before repairs complete.";
                create_failure_result(20, narrative)
            }
        }
    } else {
        // Broadcast sympathizer
        let narrative = "You arrest the sympathizer before the broadcast. ";
        narrative += "They go quietly - a true believer, but not a fighter. ";
        narrative += "The propaganda dies with them. For now.";

        create_success_result(25, 5, narrative)
    }
}

fn handle_let_broadcast(ctx, data) {
    let narrative = "You let the broadcast proceed, tracking everyone who listens. ";
    narrative += "The message spreads - but so does your surveillance. ";
    narrative += "You identify a dozen sympathizers for monitoring. ";
    narrative += "Intelligence gained, but at what cost to public trust?";

    // Mixed outcome - intel gain but reputation hit
    create_success_result(-5, 10, narrative)
}

fn on_combat_resolved(ctx, combat_result) {
    if combat_result["player_won"] {
        let narrative = "The last terrorist falls. The hostages are free, ";
        narrative += "though some need immediate medical attention. ";
        narrative += "You've won, but the cost will be counted in nightmares.";
        create_success_result(35, 20, narrative)
    } else {
        let narrative = "You're forced back. The terrorists' victory is brief - ";
        narrative += "backup arrives and ends the standoff. But the delay cost lives.";
        create_failure_result(25, narrative)
    }
}
