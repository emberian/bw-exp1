-- Seed data for starting sector and locations

-- Thornwick Sector (starting area)
INSERT OR IGNORE INTO sectors (id, name, description, danger_level, traffic_density, is_core_sector)
VALUES
    ('s0000001-0001-0001-0001-000000000001', 'Thornwick Sector',
     'A frontier sector on the edge of Compact space. Home to Thornwick Station, the largest free port in the region.',
     'moderate', 'moderate', 0);

-- Locations in Thornwick Sector
INSERT OR IGNORE INTO locations (id, sector_id, name, description, location_type, position_x, position_y, position_z, services)
VALUES
    -- Main station
    ('l0000001-0001-0001-0001-000000000001', 's0000001-0001-0001-0001-000000000001',
     'Thornwick Station',
     'Former TCF naval supply depot, now a free port. The largest station in sector.',
     'FreePort', 0.0, 0.0, 0.0,
     '["repair", "refuel", "rearm", "trade", "missions"]'),

    -- Mining facility
    ('l0000002-0002-0002-0002-000000000002', 's0000001-0001-0001-0001-000000000001',
     'Dustfall Mining Complex',
     'Automated mining operation extracting rare minerals from the asteroid belt.',
     'MiningFacility', 200.0, -150.0, 20.0,
     '["refuel", "trade"]'),

    -- Debris field
    ('l0000003-0003-0003-0003-000000000003', 's0000001-0001-0001-0001-000000000001',
     'The Wreckage',
     'Remnants of a TCF battle group. Scavengers report strange signals from the debris.',
     'DebrisField', -300.0, 100.0, -10.0,
     '[]'),

    -- Jumpgate
    ('l0000004-0004-0004-0004-000000000004', 's0000001-0001-0001-0001-000000000001',
     'Relay Point Alpha',
     'Jumpgate connection to the inner systems. Currently offline for maintenance.',
     'Jumpgate', 400.0, 0.0, 0.0,
     '[]'),

    -- Research station
    ('l0000005-0005-0005-0005-000000000005', 's0000001-0001-0001-0001-000000000001',
     'Prometheus Lab',
     'A small Illuminate research station studying Sera fragments.',
     'ResearchStation', -100.0, -200.0, 50.0,
     '["repair", "trade"]');
