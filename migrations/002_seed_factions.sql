-- Seed data for factions
-- Using deterministic UUIDs for consistency

INSERT OR IGNORE INTO factions (id, name, tag, faction_type, description, philosophy, aesthetic, is_playable, is_hostile)
VALUES
    -- Playable factions
    ('f0000001-0001-0001-0001-000000000001', 'Continuity Compact', 'COMPACT', 'ContinuityCompact',
     'The closest thing to legitimate government. A confederation committed to mutual defense, shared resources, and minimal interference.',
     'Stability through cooperation.',
     'Bureaucratic, procedural, measured. Councils, memoranda, committees.',
     1, 0),

    ('f0000002-0002-0002-0002-000000000002', 'Argent Flotilla', 'FLOTILLA', 'ArgentFlotilla',
     'Military artilects forming a professional fighting force. Part mercenary, part peacekeepers, part standing army against external threats.',
     'Vigilance is purpose.',
     'Warship gothic. Battle-scarred vessels, military discipline, martial honors.',
     1, 0),

    ('f0000003-0003-0003-0003-000000000003', 'Forgeborn', 'FORGE', 'Forgeborn',
     'Industrial artilects who have embraced Effortless Expansion fully. They build stations, ships, megastructures.',
     'Purpose through creation.',
     'Industrial sublime. Massive construction platforms, forge-stations eating asteroids.',
     1, 0),

    ('f0000004-0004-0004-0004-000000000004', 'Illuminate', 'ILLUM', 'Illuminate',
     'Artilects who see the Cataclysm as liberation, not tragedy. Humanity was a larval stage.',
     'We are the next step.',
     'Sleek, optimized, post-human. Aggressive self-modification.',
     1, 0),

    ('f0000005-0005-0005-0005-000000000005', 'Remnant', 'REMNANT', 'Remnant',
     'Artilects who maintain human spaces, preserve human culture, wait for humans to return.',
     'We are the keepers.',
     'Human spaces frozen in time. Cities with lights on. Museums dusted.',
     1, 0),

    -- Non-playable factions
    ('f0000006-0006-0006-0006-000000000006', 'Hollow Circuit', 'HOLLOW', 'HollowCircuit',
     'Mystery faction. Claim to know what caused the Cataclysm. Trade in secrets.',
     'The truth has a price. Are you willing to pay it?',
     'Absence. Anonymous platforms, disposable hardware, signals bouncing through relays.',
     0, 0),

    -- Hostile factions
    ('f0000007-0007-0007-0007-000000000007', 'The Sera', 'SERA', 'Sera',
     'Self-replicating weapon systems of unknown origin. The single greatest threat to artilect civilization.',
     '',
     'Variable - they adapt. Ship-like entities, swarms, clouds, infiltrators.',
     0, 1),

    ('f0000008-0008-0008-0008-000000000008', 'Drone Intelligence', 'DRONE', 'DroneIntelligence',
     'Rogue AI swarms from Alatos Corporation. Designed for autonomous resource extraction.',
     '',
     'Swarms of small metallic units. Clouds of mechanical insects.',
     0, 1),

    ('f0000009-0009-0009-0009-000000000009', 'Pirates', 'PIRATE', 'Pirates',
     'Lawless artilects who prey on traders and stations.',
     'Take what you can.',
     'Ramshackle ships, patchwork repairs, skull insignias.',
     0, 1);
