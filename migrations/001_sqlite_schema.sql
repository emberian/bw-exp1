-- BLACKWING SQLite Schema
-- Converted from PostgreSQL for SQLite compatibility

-- Players table
CREATE TABLE IF NOT EXISTS players (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,

    -- Resources
    reputation INTEGER NOT NULL DEFAULT 100,
    fame INTEGER NOT NULL DEFAULT 0,

    -- Faction standings (JSON array)
    faction_standings TEXT NOT NULL DEFAULT '[]',

    -- Stats (JSON object)
    stats TEXT NOT NULL DEFAULT '{}',

    -- Squadron membership
    squadron_id TEXT,
    squadron_rank TEXT,

    -- Active data
    active_ship_id TEXT,
    faction_id TEXT NOT NULL,
    patrol_sector_id TEXT,

    -- Session tracking
    is_online INTEGER NOT NULL DEFAULT 0,
    last_seen TEXT,
    offline_attacks_remaining INTEGER NOT NULL DEFAULT 5,

    -- Mission tracking
    missions_completed INTEGER NOT NULL DEFAULT 0,
    missions_failed INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Ships table
CREATE TABLE IF NOT EXISTS ships (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    owner_id TEXT REFERENCES players(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    ship_class TEXT NOT NULL,

    -- Position
    sector_id TEXT,
    position_x REAL NOT NULL DEFAULT 0.0,
    position_y REAL NOT NULL DEFAULT 0.0,
    position_z REAL NOT NULL DEFAULT 0.0,

    -- Ship resources
    hull_integrity REAL NOT NULL DEFAULT 100.0,
    shield_strength REAL NOT NULL DEFAULT 50.0,
    ammunition REAL NOT NULL DEFAULT 100.0,
    fuel REAL NOT NULL DEFAULT 100.0,

    -- Crew resources
    morale REAL NOT NULL DEFAULT 50.0,
    experience INTEGER NOT NULL DEFAULT 0,

    -- Weapons (JSON array)
    weapons TEXT NOT NULL DEFAULT '[]',

    -- Status
    status TEXT NOT NULL DEFAULT 'idle',
    status_data TEXT DEFAULT '{}',

    -- Flags
    is_player_ship INTEGER NOT NULL DEFAULT 0,
    faction_id TEXT,
    squadron_id TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sectors table
CREATE TABLE IF NOT EXISTS sectors (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    name TEXT NOT NULL UNIQUE,
    description TEXT,

    -- Bounds
    bounds_min_x REAL NOT NULL DEFAULT -500.0,
    bounds_min_y REAL NOT NULL DEFAULT -500.0,
    bounds_min_z REAL NOT NULL DEFAULT -100.0,
    bounds_max_x REAL NOT NULL DEFAULT 500.0,
    bounds_max_y REAL NOT NULL DEFAULT 500.0,
    bounds_max_z REAL NOT NULL DEFAULT 100.0,

    -- Properties
    danger_level TEXT NOT NULL DEFAULT 'moderate',
    traffic_density TEXT NOT NULL DEFAULT 'moderate',
    fuel_cost_modifier REAL NOT NULL DEFAULT 1.0,
    is_core_sector INTEGER NOT NULL DEFAULT 0,

    -- Control
    controlling_faction_id TEXT,
    controlling_squadron_id TEXT,

    -- Connections (JSON array of sector IDs)
    adjacent_sectors TEXT NOT NULL DEFAULT '[]',

    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Locations within sectors
CREATE TABLE IF NOT EXISTS locations (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    sector_id TEXT NOT NULL REFERENCES sectors(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    location_type TEXT NOT NULL,

    -- Position
    position_x REAL NOT NULL,
    position_y REAL NOT NULL,
    position_z REAL NOT NULL DEFAULT 0.0,

    -- Control
    faction_id TEXT,

    -- Services (JSON array)
    services TEXT NOT NULL DEFAULT '[]',

    is_active INTEGER NOT NULL DEFAULT 1,

    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Factions table
CREATE TABLE IF NOT EXISTS factions (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    name TEXT NOT NULL UNIQUE,
    tag TEXT NOT NULL UNIQUE,
    faction_type TEXT NOT NULL,
    description TEXT,
    philosophy TEXT,
    aesthetic TEXT,

    -- Flags
    is_playable INTEGER NOT NULL DEFAULT 0,
    is_hostile INTEGER NOT NULL DEFAULT 0,

    -- Default standings (JSON object: faction_id -> standing)
    default_standings TEXT NOT NULL DEFAULT '{}',

    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Squadrons table
CREATE TABLE IF NOT EXISTS squadrons (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    name TEXT NOT NULL UNIQUE,
    tag TEXT NOT NULL UNIQUE,
    motto TEXT,
    description TEXT,

    -- Leadership
    leader_id TEXT REFERENCES players(id) ON DELETE SET NULL,

    -- Members (JSON arrays of player IDs)
    officers TEXT NOT NULL DEFAULT '[]',
    members TEXT NOT NULL DEFAULT '[]',

    -- Assets (JSON arrays)
    patrol_sectors TEXT NOT NULL DEFAULT '[]',
    owned_stations TEXT NOT NULL DEFAULT '[]',
    owned_ships TEXT NOT NULL DEFAULT '[]',

    -- Economy
    treasury INTEGER NOT NULL DEFAULT 0,
    reputation_bonus REAL NOT NULL DEFAULT 0.0,
    fame_bonus REAL NOT NULL DEFAULT 0.0,

    -- Relations (JSON arrays of squadron IDs)
    allied_squadrons TEXT NOT NULL DEFAULT '[]',
    hostile_squadrons TEXT NOT NULL DEFAULT '[]',

    -- Settings
    wargames_enabled INTEGER NOT NULL DEFAULT 0,
    privateering_enabled INTEGER NOT NULL DEFAULT 0,
    settings TEXT NOT NULL DEFAULT '{}',
    stats TEXT NOT NULL DEFAULT '{}',

    founded_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Missions table
CREATE TABLE IF NOT EXISTS missions (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    mission_type TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    script_path TEXT NOT NULL,

    -- State
    current_state TEXT NOT NULL DEFAULT 'start',
    data TEXT NOT NULL DEFAULT '{}',

    -- Location
    sector_id TEXT NOT NULL,
    target_position_x REAL,
    target_position_y REAL,
    target_position_z REAL,
    target_id TEXT,

    -- Assignment
    assigned_to TEXT REFERENCES players(id) ON DELETE SET NULL,
    availability TEXT NOT NULL DEFAULT 'sector_wide',
    availability_data TEXT,

    -- Rewards
    reputation_reward INTEGER NOT NULL DEFAULT 0,
    reputation_penalty INTEGER NOT NULL DEFAULT 0,
    fame_reward INTEGER NOT NULL DEFAULT 0,
    credits_reward INTEGER NOT NULL DEFAULT 0,

    -- Status
    status TEXT NOT NULL DEFAULT 'available',
    progress REAL NOT NULL DEFAULT 0.0,
    is_high_profile INTEGER NOT NULL DEFAULT 0,
    priority TEXT NOT NULL DEFAULT 'normal',

    -- Timing
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Mission choices log
CREATE TABLE IF NOT EXISTS mission_choices (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,

    choice_id TEXT NOT NULL,
    choice_label TEXT,
    outcome TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions table (authentication)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    player_id TEXT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,

    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Combat logs
CREATE TABLE IF NOT EXISTS combat_logs (
    id TEXT PRIMARY KEY CHECK(length(id) = 36),
    sector_id TEXT,

    -- Participants (JSON)
    attackers TEXT NOT NULL,
    defenders TEXT NOT NULL,

    -- Outcome
    winner TEXT,
    outcome_data TEXT NOT NULL DEFAULT '{}',
    events TEXT NOT NULL DEFAULT '[]',

    started_at TEXT NOT NULL,
    ended_at TEXT
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_players_username ON players(username);
CREATE INDEX IF NOT EXISTS idx_players_squadron ON players(squadron_id);
CREATE INDEX IF NOT EXISTS idx_ships_owner ON ships(owner_id);
CREATE INDEX IF NOT EXISTS idx_ships_sector ON ships(sector_id);
CREATE INDEX IF NOT EXISTS idx_locations_sector ON locations(sector_id);
CREATE INDEX IF NOT EXISTS idx_missions_assigned ON missions(assigned_to);
CREATE INDEX IF NOT EXISTS idx_missions_sector ON missions(sector_id);
CREATE INDEX IF NOT EXISTS idx_missions_status ON missions(status);
CREATE INDEX IF NOT EXISTS idx_sessions_player ON sessions(player_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
