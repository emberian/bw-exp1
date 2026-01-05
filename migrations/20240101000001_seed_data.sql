-- BLACKWING Seed Data
-- Initial factions and starting sector

-- Insert canonical factions
INSERT INTO factions (id, name, faction_type, description, is_hostile, default_standing, relations) VALUES
    (
        uuid_generate_v4(),
        'Continuity Compact',
        'government',
        'The legitimate governing body maintaining order across known space. Bureaucratic but stable.',
        false,
        50,
        '{"Argent Flotilla": 75, "Forgeborn Collective": 50, "Illuminate": 25, "Remnant": 40, "Hollow Circuit": -25}'
    ),
    (
        uuid_generate_v4(),
        'Argent Flotilla',
        'military',
        'The military arm dedicated to Sera defense. Disciplined and honor-bound.',
        false,
        25,
        '{"Continuity Compact": 75, "Forgeborn Collective": 60, "Illuminate": 10, "Remnant": 30, "Hollow Circuit": -50}'
    ),
    (
        uuid_generate_v4(),
        'Forgeborn Collective',
        'industrial',
        'Industrial faction focused on construction and resource extraction. Always buying materials.',
        false,
        25,
        '{"Continuity Compact": 50, "Argent Flotilla": 60, "Illuminate": 30, "Remnant": 20, "Hollow Circuit": 0}'
    ),
    (
        uuid_generate_v4(),
        'Illuminate',
        'religious',
        'Philosophical order believing in transcendence through pure thought. Mysterious and esoteric.',
        false,
        0,
        '{"Continuity Compact": 25, "Argent Flotilla": 10, "Forgeborn Collective": 30, "Remnant": 60, "Hollow Circuit": 40}'
    ),
    (
        uuid_generate_v4(),
        'Remnant',
        'preservers',
        'Dedicated to preserving human artifacts and memory. Collectors and archivists.',
        false,
        0,
        '{"Continuity Compact": 40, "Argent Flotilla": 30, "Forgeborn Collective": 20, "Illuminate": 60, "Hollow Circuit": 25}'
    ),
    (
        uuid_generate_v4(),
        'Hollow Circuit',
        'criminal',
        'Information brokers and smugglers operating in the shadows. Trust no one.',
        false,
        -25,
        '{"Continuity Compact": -25, "Argent Flotilla": -50, "Forgeborn Collective": 0, "Illuminate": 40, "Remnant": 25}'
    ),
    (
        uuid_generate_v4(),
        'The Sera',
        'enemy',
        'Self-replicating weapons of unknown origin. Existential threat to all artilect civilization.',
        true,
        -100,
        '{}'
    ),
    (
        uuid_generate_v4(),
        'Drone Intelligences',
        'enemy',
        'Rogue harvester AIs that strip ships and stations for resources. Mindless but dangerous.',
        true,
        -75,
        '{}'
    );

-- Insert starting sector: Thornwick
INSERT INTO sectors (id, name, description, min_x, min_y, max_x, max_y, danger_level, traffic_density, controlling_faction) VALUES
    (
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Thornwick Sector',
        'A moderately developed sector on the edge of Compact space. Home to Thornwick Station, a major trading hub and Space Guard outpost.',
        -1000.0,
        -1000.0,
        1000.0,
        1000.0,
        'moderate',
        'normal',
        'Continuity Compact'
    );

-- Insert locations in Thornwick Sector
INSERT INTO locations (id, sector_id, name, location_type, position_x, position_y, owner_faction, services) VALUES
    (
        '00000000-0000-0000-0000-000000000010'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Thornwick Station',
        'station',
        0.0,
        0.0,
        'Continuity Compact',
        '["refuel", "rearm", "repair", "trade", "mission_board", "shore_leave"]'
    ),
    (
        '00000000-0000-0000-0000-000000000011'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Forgeborn Depot',
        'station',
        -500.0,
        300.0,
        'Forgeborn Collective',
        '["refuel", "repair", "trade"]'
    ),
    (
        '00000000-0000-0000-0000-000000000012'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Remnant Archive',
        'station',
        400.0,
        -200.0,
        'Remnant',
        '["trade", "mission_board"]'
    ),
    (
        '00000000-0000-0000-0000-000000000013'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Jump Gate Alpha',
        'jump_gate',
        800.0,
        600.0,
        'Continuity Compact',
        '[]'
    ),
    (
        '00000000-0000-0000-0000-000000000014'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Jump Gate Beta',
        'jump_gate',
        -700.0,
        -500.0,
        'Continuity Compact',
        '[]'
    ),
    (
        '00000000-0000-0000-0000-000000000015'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Asteroid Field Kappa',
        'asteroid_field',
        300.0,
        500.0,
        NULL,
        '[]'
    ),
    (
        '00000000-0000-0000-0000-000000000016'::uuid,
        '00000000-0000-0000-0000-000000000001'::uuid,
        'Nebula Shroud',
        'nebula',
        -300.0,
        -300.0,
        NULL,
        '[]'
    );
