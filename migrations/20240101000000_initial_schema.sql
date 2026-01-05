-- BLACKWING Initial Database Schema
-- Core tables for game persistence

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Players table
CREATE TABLE players (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(32) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,

    -- Resources
    reputation INTEGER NOT NULL DEFAULT 100,
    fame INTEGER NOT NULL DEFAULT 0,

    -- Faction standings (JSONB for flexibility)
    faction_standings JSONB NOT NULL DEFAULT '{}',

    -- Stats
    missions_completed INTEGER NOT NULL DEFAULT 0,
    ships_destroyed INTEGER NOT NULL DEFAULT 0,
    ships_lost INTEGER NOT NULL DEFAULT 0,
    total_distance DOUBLE PRECISION NOT NULL DEFAULT 0.0,

    -- Squadron membership
    squadron_id UUID,
    squadron_rank VARCHAR(32),

    -- Session tracking
    current_sector_id UUID,
    last_online TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Ships table
CREATE TABLE ships (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    name VARCHAR(64) NOT NULL,
    ship_class VARCHAR(32) NOT NULL,

    -- Position
    sector_id UUID,
    position_x DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    position_y DOUBLE PRECISION NOT NULL DEFAULT 0.0,

    -- Ship resources
    hull REAL NOT NULL DEFAULT 100.0,
    max_hull REAL NOT NULL DEFAULT 100.0,
    shields REAL NOT NULL DEFAULT 50.0,
    max_shields REAL NOT NULL DEFAULT 50.0,

    -- Crew resources
    ammunition REAL NOT NULL DEFAULT 100.0,
    fuel REAL NOT NULL DEFAULT 100.0,
    morale REAL NOT NULL DEFAULT 50.0,
    experience INTEGER NOT NULL DEFAULT 0,

    -- Ship stats
    max_speed REAL NOT NULL DEFAULT 10.0,
    maneuverability REAL NOT NULL DEFAULT 1.0,
    sensor_range REAL NOT NULL DEFAULT 100.0,
    cargo_capacity INTEGER NOT NULL DEFAULT 100,

    -- Weapons (JSONB array)
    weapons JSONB NOT NULL DEFAULT '[]',

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'idle',
    is_docked BOOLEAN NOT NULL DEFAULT false,
    docked_at UUID,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Sectors table
CREATE TABLE sectors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) NOT NULL UNIQUE,
    description TEXT,

    -- Bounds
    min_x DOUBLE PRECISION NOT NULL,
    min_y DOUBLE PRECISION NOT NULL,
    max_x DOUBLE PRECISION NOT NULL,
    max_y DOUBLE PRECISION NOT NULL,

    -- Sector properties
    danger_level VARCHAR(16) NOT NULL DEFAULT 'moderate',
    traffic_density VARCHAR(16) NOT NULL DEFAULT 'normal',
    controlling_faction VARCHAR(32),

    -- Connected sectors (JSONB array of UUIDs)
    connections JSONB NOT NULL DEFAULT '[]',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Locations within sectors (stations, jump points, etc.)
CREATE TABLE locations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sector_id UUID NOT NULL REFERENCES sectors(id) ON DELETE CASCADE,
    name VARCHAR(64) NOT NULL,
    location_type VARCHAR(32) NOT NULL,

    -- Position within sector
    position_x DOUBLE PRECISION NOT NULL,
    position_y DOUBLE PRECISION NOT NULL,

    -- Owner/controller
    owner_faction VARCHAR(32),
    owner_squadron_id UUID,

    -- Station services (JSONB array if applicable)
    services JSONB NOT NULL DEFAULT '[]',

    -- Properties
    properties JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Factions table
CREATE TABLE factions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) NOT NULL UNIQUE,
    faction_type VARCHAR(32) NOT NULL,
    description TEXT,

    -- Disposition
    is_hostile BOOLEAN NOT NULL DEFAULT false,
    default_standing INTEGER NOT NULL DEFAULT 0,

    -- Relations with other factions (JSONB map)
    relations JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Missions table
CREATE TABLE missions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Mission definition
    script_id VARCHAR(128) NOT NULL,
    mission_type VARCHAR(32) NOT NULL,
    title VARCHAR(128) NOT NULL,
    description TEXT,

    -- Assignment
    assigned_player_id UUID REFERENCES players(id) ON DELETE SET NULL,
    sector_id UUID REFERENCES sectors(id) ON DELETE CASCADE,
    issuing_faction VARCHAR(32),

    -- Status
    status VARCHAR(32) NOT NULL DEFAULT 'available',
    current_state VARCHAR(64),
    state_data JSONB NOT NULL DEFAULT '{}',

    -- Rewards
    reputation_reward INTEGER NOT NULL DEFAULT 0,
    fame_reward INTEGER NOT NULL DEFAULT 0,

    -- Timing
    available_until TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Mission choices log (for replay/debugging)
CREATE TABLE mission_choices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mission_id UUID NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,

    choice_id VARCHAR(64) NOT NULL,
    choice_label VARCHAR(128),
    outcome JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Squadrons table
CREATE TABLE squadrons (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(64) NOT NULL UNIQUE,
    tag VARCHAR(8) NOT NULL UNIQUE,
    description TEXT,

    -- Leadership
    leader_id UUID REFERENCES players(id) ON DELETE SET NULL,

    -- Stats
    total_reputation INTEGER NOT NULL DEFAULT 0,
    total_fame INTEGER NOT NULL DEFAULT 0,
    member_count INTEGER NOT NULL DEFAULT 0,

    -- Alliances (JSONB array of squadron UUIDs)
    allies JSONB NOT NULL DEFAULT '[]',
    enemies JSONB NOT NULL DEFAULT '[]',

    -- Settings
    settings JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Squadron buildings
CREATE TABLE squadron_buildings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    squadron_id UUID NOT NULL REFERENCES squadrons(id) ON DELETE CASCADE,
    location_id UUID NOT NULL REFERENCES locations(id) ON DELETE CASCADE,

    building_type VARCHAR(32) NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,

    -- Building data
    properties JSONB NOT NULL DEFAULT '{}',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Combat engagements log
CREATE TABLE combat_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    sector_id UUID REFERENCES sectors(id) ON DELETE SET NULL,

    -- Participants (JSONB arrays of ship/player info)
    attackers JSONB NOT NULL,
    defenders JSONB NOT NULL,

    -- Outcome
    winner VARCHAR(16), -- 'attacker', 'defender', 'draw'
    outcome_data JSONB NOT NULL DEFAULT '{}',

    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,

    -- Full combat log (JSONB array of events)
    events JSONB NOT NULL DEFAULT '[]'
);

-- Player sessions (for auth tokens)
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,

    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add foreign key for ships.sector_id after sectors table exists
ALTER TABLE ships ADD CONSTRAINT fk_ships_sector
    FOREIGN KEY (sector_id) REFERENCES sectors(id) ON DELETE SET NULL;

-- Add foreign key for ships.docked_at to locations
ALTER TABLE ships ADD CONSTRAINT fk_ships_docked_at
    FOREIGN KEY (docked_at) REFERENCES locations(id) ON DELETE SET NULL;

-- Add foreign key for players.squadron_id
ALTER TABLE players ADD CONSTRAINT fk_players_squadron
    FOREIGN KEY (squadron_id) REFERENCES squadrons(id) ON DELETE SET NULL;

-- Add foreign key for players.current_sector_id
ALTER TABLE players ADD CONSTRAINT fk_players_sector
    FOREIGN KEY (current_sector_id) REFERENCES sectors(id) ON DELETE SET NULL;

-- Indexes for common queries
CREATE INDEX idx_ships_owner ON ships(owner_id);
CREATE INDEX idx_ships_sector ON ships(sector_id);
CREATE INDEX idx_locations_sector ON locations(sector_id);
CREATE INDEX idx_missions_player ON missions(assigned_player_id);
CREATE INDEX idx_missions_sector ON missions(sector_id);
CREATE INDEX idx_missions_status ON missions(status);
CREATE INDEX idx_squadron_buildings_squadron ON squadron_buildings(squadron_id);
CREATE INDEX idx_sessions_player ON sessions(player_id);
CREATE INDEX idx_sessions_token ON sessions(token_hash);
