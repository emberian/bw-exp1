// Asteroid Threat Mission Script
// A rogue asteroid threatens a civilian installation.
// Time-critical mission with multiple solution paths.
//
// States: start -> assessment -> (demolition|evacuation|deflection) -> resolution

fn on_start(ctx) {
    // Determine target type and asteroid size
    let targets = ["mining station", "orbital habitat", "research platform", "fuel depot"];
    let target = targets[rand_int(0, 3)];

    let size_roll = rand();
    let asteroid_size = if size_roll < 0.5 {
        "small"     // Manageable with weapons
    } else if size_roll < 0.85 {
        "medium"    // Need creative solutions
    } else {
        "large"     // Evacuation may be only option
    };

    let population = if target == "mining station" {
        rand_int(50, 200)
    } else if target == "orbital habitat" {
        rand_int(500, 2000)
    } else if target == "research platform" {
        rand_int(20, 80)
    } else {
        rand_int(15, 40)
    };

    let narrative = "PRIORITY ALERT: Collision course detected. ";
    narrative += "A " + asteroid_size + " asteroid is on direct trajectory with " + target + " Artemis-7. ";
    narrative += population + " souls aboard. Impact in approximately 4 hours. ";
    narrative += "You're the closest vessel.";

    let choices = [];

    choices.push(create_choice("assess", "Move to intercept for close assessment"));

    if asteroid_size == "small" {
        choices.push(create_choice("weapons", "Engage with ship weapons immediately"));
    }

    if meets_requirement(ctx, "MinExperience", 60) {
        choices.push(create_choice("calculate", "Run trajectory calculations for deflection options"));
    }

    choices.push(create_choice("evacuate_start", "Begin coordinating civilian evacuation"));

    let data = #{
        target: target,
        asteroid_size: asteroid_size,
        population: population,
        time_remaining: 240,  // minutes
        assessed: false
    };

    create_choice_result("alert", narrative, choices)
}

fn on_choice(ctx, choice_id) {
    let data = get_mission_data(ctx);

    if choice_id == "assess" {
        handle_assess(ctx, data)
    } else if choice_id == "weapons" {
        handle_weapons(ctx, data)
    } else if choice_id == "calculate" {
        handle_calculate(ctx, data)
    } else if choice_id == "evacuate_start" {
        handle_evacuate_start(ctx, data)
    } else if choice_id == "precise_strike" {
        handle_precise_strike(ctx, data)
    } else if choice_id == "sustained_fire" {
        handle_sustained_fire(ctx, data)
    } else if choice_id == "deflect_burn" {
        handle_deflect_burn(ctx, data)
    } else if choice_id == "push" {
        handle_push(ctx, data)
    } else if choice_id == "fragment" {
        handle_fragment(ctx, data)
    } else if choice_id == "coordinate_evac" {
        handle_coordinate_evac(ctx, data)
    } else if choice_id == "tractor" {
        handle_tractor(ctx, data)
    } else {
        create_failure_result(10, "Time wasted. The asteroid strikes.")
    }
}

fn handle_assess(ctx, data) {
    let asteroid_size = data["asteroid_size"];
    let target = data["target"];

    let narrative = "Close-range scans reveal the asteroid's composition. ";

    if asteroid_size == "small" {
        narrative += "Primarily iron-nickel with some silicate inclusions. ";
        narrative += "Approximately 50 meters diameter. Your weapons can destroy this, ";
        narrative += "but you'll need to hit specific stress fractures.";

        let choices = [];
        choices.push(create_choice("precise_strike", "Target weak points with focused fire"));
        choices.push(create_choice("sustained_fire", "Full weapons discharge"));
        choices.push(create_choice("evacuate_start", "Evacuation is safer"));

        create_choice_result("small_assessed", narrative, choices)

    } else if asteroid_size == "medium" {
        narrative += "200 meters diameter. Too large to destroy outright, but sensors detect ";
        narrative += "a large ice deposit on the trailing edge. Heating that ice could ";
        narrative += "create thrust - enough to deflect the trajectory.";

        let choices = [];

        if meets_requirement(ctx, "MinAmmo", 50.0) {
            choices.push(create_choice("deflect_burn", "Target the ice deposit with sustained fire"));
        } else {
            choices.push(create_choice_with_req("deflect_burn", "Burn ice for deflection", "MinAmmo", 50.0));
        }

        choices.push(create_choice("push", "Use your ship to physically push the asteroid"));
        choices.push(create_choice("evacuate_start", "Focus on evacuation"));

        create_choice_result("medium_assessed", narrative, choices)

    } else {
        narrative += "Half a kilometer of rock and metal. This monster will destroy ";
        narrative += "anything in its path. There's no stopping it. ";
        narrative += "The only option is evacuation.";

        let choices = [];
        choices.push(create_choice("coordinate_evac", "Coordinate emergency evacuation"));
        choices.push(create_choice("fragment", "Attempt to fragment it anyway"));

        create_choice_result("large_assessed", narrative, choices)
    }
}

fn handle_weapons(ctx, data) {
    let asteroid_size = data["asteroid_size"];

    if asteroid_size == "small" {
        let experience = get_crew_experience(ctx);
        let success = 0.5 + experience_success_chance(0.2, experience);

        if rand() < success {
            let narrative = "Your weapons officer finds the mark. The asteroid breaks apart, ";
            narrative += "fragments scattering harmlessly into space. The " + data["target"] + " is safe.";

            create_success_result(35, 15, narrative)
        } else {
            let narrative = "Your fire isn't coordinated enough. The asteroid breaks into large chunks - ";
            narrative += "still dangerous. You've made things worse.";

            let choices = [];
            choices.push(create_choice("fragment", "Try to break up the chunks further"));
            choices.push(create_choice("evacuate_start", "Evacuate - no time left"));

            create_choice_result("made_worse", narrative, choices)
        }
    } else {
        let narrative = "Your weapons barely scratch the surface. This rock is too big. ";
        narrative += "You've wasted precious time.";

        let choices = [];
        choices.push(create_choice("assess", "Scan for vulnerabilities"));
        choices.push(create_choice("evacuate_start", "Begin evacuation"));

        let new_data = #{
            target: data["target"],
            asteroid_size: data["asteroid_size"],
            population: data["population"],
            time_remaining: data["time_remaining"] - 30,
            assessed: data["assessed"]
        };

        create_choice_result_with_data("weapons_failed", narrative, choices, new_data)
    }
}

fn handle_calculate(ctx, data) {
    let experience = get_crew_experience(ctx);
    let narrative = "Your crew runs the numbers. ";

    if data["asteroid_size"] == "small" {
        narrative += "A 0.3 degree deflection is sufficient. Your weapons can achieve this ";
        narrative += "if you target the right stress points.";

        let choices = [];
        choices.push(create_choice("precise_strike", "Execute precise strike pattern"));

        create_choice_result("calculated_small", narrative, choices)

    } else if data["asteroid_size"] == "medium" {
        narrative += "You need a 0.8 degree deflection. Options: ";
        narrative += "1) Sustained burn on the ice deposit. 2) Physical push with your ship. ";
        narrative += "3) Tractor beam deflection if you can hold it long enough.";

        let choices = [];
        choices.push(create_choice("deflect_burn", "Ice burn deflection"));
        choices.push(create_choice("push", "Physical push"));

        if meets_requirement(ctx, "MinFuel", 40.0) {
            choices.push(create_choice("tractor", "Sustained tractor beam"));
        }

        create_choice_result("calculated_medium", narrative, choices)

    } else {
        narrative += "There's no deflecting this. Even fragmenting it will just create ";
        narrative += "a shotgun blast. Evacuation is the only viable option.";

        let choices = [];
        choices.push(create_choice("coordinate_evac", "Coordinate evacuation"));

        create_choice_result("calculated_large", narrative, choices)
    }
}

fn handle_precise_strike(ctx, data) {
    let experience = get_crew_experience(ctx);
    let ammo = ctx["ship_ammo"];

    let success_chance = 0.4 + experience_success_chance(0.35, experience);

    if ammo < 30.0 {
        success_chance -= 0.2;  // Low ammo penalty
    }

    if rand() < success_chance {
        let narrative = "Your weapons officer threads the needle. Three shots, three critical fractures. ";
        narrative += "The asteroid splits along stress lines, fragments tumbling away from the " + data["target"] + ". ";
        narrative += "Cheers erupt over the comms from the station.";

        create_success_result(40, 20, narrative)
    } else {
        let narrative = "The shots connect, but the asteroid holds together. ";
        narrative += "It's tumbling now, harder to target. Time is running out.";

        let choices = [];
        choices.push(create_choice("sustained_fire", "Full weapons discharge"));
        choices.push(create_choice("evacuate_start", "Abandon demolition - evacuate"));

        create_choice_result("precise_failed", narrative, choices)
    }
}

fn handle_sustained_fire(ctx, data) {
    let narrative = "You empty your weapons into the asteroid. It breaks apart - ";

    if rand() < 0.6 {
        narrative += "and the fragments scatter wide, missing the station entirely. ";
        narrative += "Your ammunition is depleted, but the " + data["target"] + " is safe.";

        // Success but costs all ammo
        create_success_with_cost_result(35, 15, -80.0, 0.0, narrative)
    } else {
        narrative += "but several large chunks maintain trajectory. ";
        narrative += "Partial impact. Casualties are reduced, but not prevented.";

        // Partial success
        create_success_result(15, 5, narrative)
    }
}

fn handle_deflect_burn(ctx, data) {
    let experience = get_crew_experience(ctx);
    let success = 0.5 + experience_success_chance(0.3, experience);

    if rand() < success {
        let narrative = "Your sustained fire turns the ice deposit into a massive jet of steam. ";
        narrative += "The asteroid's trajectory shifts degree by degree. ";
        narrative += "After forty agonizing minutes, it clears the " + data["target"] + " by 200 meters. ";
        narrative += "Close enough that the station crew can see individual craters.";

        create_success_with_cost_result(45, 25, -50.0, 0.0, narrative)
    } else {
        let narrative = "The ice burns off, but the deflection isn't enough. ";
        narrative += "You've slowed it, bought time. But not stopped it.";

        let choices = [];
        choices.push(create_choice("push", "Use your ship to push it the rest of the way"));
        choices.push(create_choice("coordinate_evac", "Begin emergency evacuation"));

        create_choice_result("partial_deflection", narrative, choices)
    }
}

fn handle_push(ctx, data) {
    let narrative = "You match velocity with the asteroid and engage thrusters. ";
    narrative += "Your ship groans under the stress. Hull integrity warnings flash. ";

    let hull = ctx["ship_hull"];

    if hull > 60.0 {
        narrative += "But it works. Slowly, impossibly, the asteroid's path changes. ";
        narrative += "Your hull is battered, your fuel nearly gone, but the " + data["target"] + " will survive.";

        // Success with damage
        create_success_with_cost_result(50, 30, -30.0, -25.0, narrative)
    } else {
        narrative += "Your hull can't take it. You're forced to break off. ";
        narrative += "The asteroid continues on, barely affected.";

        let choices = [];
        choices.push(create_choice("coordinate_evac", "Evacuate - it's the only option"));

        create_choice_result("push_failed", narrative, choices)
    }
}

fn handle_tractor(ctx, data) {
    let narrative = "You lock the tractor beam and begin the long process of deflection. ";
    narrative += "Your fuel reserves drain steadily as you fight against thousands of tons of rock. ";

    if rand() < 0.7 {
        narrative += "It works. Barely. Your fuel tanks are nearly dry, but the asteroid ";
        narrative += "sails past the " + data["target"] + " and into empty space.";

        create_success_with_cost_result(45, 20, -70.0, 0.0, narrative)
    } else {
        narrative += "The beam can't maintain lock. The asteroid is too massive. ";
        narrative += "You've burned fuel for nothing.";

        let choices = [];
        choices.push(create_choice("coordinate_evac", "Emergency evacuation"));

        create_choice_result("tractor_failed", narrative, choices)
    }
}

fn handle_evacuate_start(ctx, data) {
    let population = data["population"];
    let target = data["target"];

    let narrative = "You broadcast evacuation orders and begin coordinating rescue efforts. ";

    if population < 100 {
        narrative += "The " + target + " is small. Evacuation proceeds smoothly. ";
        narrative += "All " + population + " personnel are clear when the asteroid hits.";
        narrative += " The station is destroyed, but every life is saved.";

        create_success_result(30, 10, narrative)
    } else {
        narrative += "The population is large. You won't save everyone this way. ";
        narrative += "But you might save most.";

        let choices = [];
        choices.push(create_choice("coordinate_evac", "Focus entirely on evacuation"));
        choices.push(create_choice("assess", "Look for other options while evacuation proceeds"));

        create_choice_result("evac_started", narrative, choices)
    }
}

fn handle_coordinate_evac(ctx, data) {
    let population = data["population"];
    let experience = get_crew_experience(ctx);

    let evac_efficiency = 0.7 + experience_success_chance(0.2, experience);
    let pop_float = population + 0.0; // Convert to float
    let saved = (pop_float * evac_efficiency).to_int();
    let lost = population - saved;

    let narrative = "You coordinate the evacuation with military precision. ";
    narrative += saved + " of " + population + " personnel are evacuated before impact. ";

    if lost == 0 {
        narrative += "Every single life saved. The station is gone, but nothing irreplaceable was lost.";
        create_success_result(40, 20, narrative)
    } else if lost < 10 {
        narrative += lost + " lives lost - workers who refused to leave their posts. ";
        narrative += "You did everything possible.";
        create_success_result(30, 15, narrative)
    } else {
        narrative += lost + " souls. Their names will be remembered. ";
        narrative += "You saved who you could.";
        create_success_result(20, 10, narrative)
    }
}

fn handle_fragment(ctx, data) {
    let narrative = "Against all logic, you attack the massive asteroid with everything you have. ";
    narrative += "It fragments - into a dozen deadly projectiles. ";
    narrative += "The " + data["target"] + " takes multiple impacts. ";
    narrative += "Catastrophic damage. You made it worse.";

    create_failure_result(30, narrative)
}
