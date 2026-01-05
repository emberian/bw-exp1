//! Game state mirroring the frontend GameState
//!
//! This module contains all game state received from the server.
//! Some fields may not be displayed yet but are kept for future features.

#![allow(dead_code)]

use std::collections::VecDeque;

use bw_shared::{
    dto::*,
    messages::{ChatChannel, ServerMessage},
};
use uuid::Uuid;

/// Information about a ship in the sector
#[derive(Debug, Clone)]
pub struct ShipInfo {
    pub id: Uuid,
    pub name: String,
    pub ship_class: String,
    pub position: (f64, f64),
    pub hull_percent: f32,
    pub shield_percent: f32,
    pub is_player: bool,
    pub is_hostile: bool,
    pub status: String,
    pub faction_tag: Option<String>,
}

impl From<ShipDto> for ShipInfo {
    fn from(dto: ShipDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            ship_class: dto.ship_class,
            position: (dto.position.x, dto.position.y),
            hull_percent: dto.hull_percent,
            shield_percent: dto.shield_percent,
            is_player: dto.is_player,
            is_hostile: dto.is_hostile,
            status: dto.status,
            faction_tag: dto.faction_tag,
        }
    }
}

/// Information about a location in the sector
#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub id: Uuid,
    pub name: String,
    pub location_type: String,
    pub position: (f64, f64),
    pub faction_tag: Option<String>,
    pub services: Vec<String>,
}

impl From<LocationDto> for LocationInfo {
    fn from(dto: LocationDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            location_type: dto.location_type,
            position: (dto.position.x, dto.position.y),
            faction_tag: dto.faction_tag,
            services: dto.services,
        }
    }
}

/// Information about a sector
#[derive(Debug, Clone)]
pub struct SectorInfo {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
    pub controlling_faction: Option<String>,
}

impl From<&SectorDto> for SectorInfo {
    fn from(dto: &SectorDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name.clone(),
            danger_level: dto.danger_level.clone(),
            controlling_faction: dto.controlling_faction.clone(),
        }
    }
}

/// Adjacent sector info
#[derive(Debug, Clone)]
pub struct AdjacentSector {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
}

impl From<AdjacentSectorDto> for AdjacentSector {
    fn from(dto: AdjacentSectorDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            danger_level: dto.danger_level,
        }
    }
}

/// Mission information
#[derive(Debug, Clone)]
pub struct MissionInfo {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub mission_type: String,
    pub status: String,
    pub priority: String,
    pub reputation_reward: i32,
    pub fame_reward: i32,
    pub is_high_profile: bool,
    pub expires_in_seconds: Option<u32>,
    pub progress: f32,
    pub can_accept: bool,
}

impl From<MissionDto> for MissionInfo {
    fn from(dto: MissionDto) -> Self {
        Self {
            id: dto.id,
            title: dto.title,
            description: dto.description,
            mission_type: dto.mission_type,
            status: dto.status,
            priority: dto.priority,
            reputation_reward: dto.reputation_reward,
            fame_reward: dto.fame_reward,
            is_high_profile: dto.is_high_profile,
            expires_in_seconds: dto.expires_in_seconds,
            progress: dto.progress,
            can_accept: dto.can_accept,
        }
    }
}

/// Chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub sender_id: Uuid,
    pub sender_name: String,
    pub message: String,
    pub channel: ChatChannel,
    pub timestamp: u64,
}

/// Combat event for the combat log
#[derive(Debug, Clone)]
pub struct CombatLogEntry {
    pub event_type: String,
    pub attacker_name: String,
    pub target_name: String,
    pub damage: Option<f32>,
    pub hit: bool,
    pub message: String,
}

impl From<CombatEventDto> for CombatLogEntry {
    fn from(dto: CombatEventDto) -> Self {
        Self {
            event_type: dto.event_type,
            attacker_name: dto.attacker_name,
            target_name: dto.target_name,
            damage: dto.damage,
            hit: dto.hit,
            message: dto.message,
        }
    }
}

/// Active combat state
#[derive(Debug, Clone)]
pub struct CombatState {
    pub engagement_id: Uuid,
    pub round: u32,
    pub is_resolved: bool,
    pub winner: Option<String>,
    pub events: Vec<CombatLogEntry>,
}

/// Squadron info
#[derive(Debug, Clone)]
pub struct SquadronInfo {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub motto: Option<String>,
    pub leader_name: String,
    pub member_count: u32,
}

impl From<SquadronDto> for SquadronInfo {
    fn from(dto: SquadronDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            tag: dto.tag,
            motto: dto.motto,
            leader_name: dto.leader_name,
            member_count: dto.member_count,
        }
    }
}

/// Mission choice presented to player
#[derive(Debug, Clone)]
pub struct MissionChoice {
    pub mission_id: Uuid,
    pub description: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub is_available: bool,
    pub requirement_text: Option<String>,
}

impl From<ChoiceDto> for Choice {
    fn from(dto: ChoiceDto) -> Self {
        Self {
            id: dto.id,
            text: dto.text,
            is_available: dto.is_available,
            requirement_text: dto.requirement_text,
        }
    }
}

/// Game state received from server
#[derive(Debug, Default)]
pub struct GameState {
    // Connection state
    pub connected: bool,
    pub player_id: Option<Uuid>,
    pub username: String,
    pub server_tick: u64,

    // Player resources
    pub reputation: i32,
    pub fame: i32,
    pub ammunition: f32,
    pub fuel: f32,
    pub morale: f32,
    pub experience: i32,

    // Ship state
    pub ship_id: Option<Uuid>,
    pub ship_name: String,
    pub ship_class: String,
    pub ship_hull: f32,
    pub ship_shields: f32,
    pub ship_max_hull: f32,
    pub ship_max_shields: f32,
    pub ship_status: String,
    pub position: (f64, f64),

    // Sector
    pub sector: Option<SectorInfo>,
    pub locations: Vec<LocationInfo>,
    pub ships: Vec<ShipInfo>,
    pub adjacent_sectors: Vec<AdjacentSector>,

    // Missions
    pub available_missions: Vec<MissionInfo>,
    pub active_mission: Option<MissionInfo>,
    pub mission_choice: Option<MissionChoice>,

    // Combat
    pub combat: Option<CombatState>,
    pub combat_log: VecDeque<CombatLogEntry>,

    // Chat
    pub chat_messages: VecDeque<ChatMessage>,

    // Squadron
    pub squadron: Option<SquadronInfo>,

    // Admin state
    pub is_admin: bool,

    // Performance metrics (debug)
    pub tick_metrics: Option<TickMetricsDto>,
    pub tick_metrics_history: Option<TickMetricsHistoryDto>,
}

impl GameState {
    /// Process a server message and update state
    pub fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::AuthResult {
                success,
                player_id,
                error,
            } => {
                if success {
                    self.player_id = player_id;
                } else if let Some(err) = error {
                    tracing::error!("Auth failed: {}", err);
                }
            }

            ServerMessage::InitialState {
                player,
                ship,
                sector,
                ships,
                missions,
            } => {
                // Player info
                self.player_id = Some(player.id);
                self.username = player.username;
                self.reputation = player.reputation;
                self.fame = player.fame;
                self.is_admin = player.is_admin;

                // Ship info
                self.ship_id = Some(ship.id);
                self.ship_name = ship.name;
                self.ship_class = ship.ship_class;
                self.ship_hull = ship.hull_percent;
                self.ship_shields = ship.shield_percent;
                self.ship_status = ship.status;
                self.position = (ship.position.x, ship.position.y);

                // Sector info
                self.sector = Some(SectorInfo::from(&sector));
                self.locations = sector.locations.into_iter().map(LocationInfo::from).collect();
                self.adjacent_sectors = sector
                    .adjacent_sectors
                    .into_iter()
                    .map(AdjacentSector::from)
                    .collect();

                // Ships in sector
                self.ships = ships.into_iter().map(ShipInfo::from).collect();

                // Missions
                self.available_missions = missions
                    .into_iter()
                    .filter(|m| m.status == "Available")
                    .map(MissionInfo::from)
                    .collect();
                self.active_mission = None; // Will be set by mission with "Active" status
            }

            ServerMessage::StateUpdate {
                tick,
                ship_updates,
                ship_spawns,
                ship_despawns,
                mission_updates,
                events,
            } => {
                self.server_tick = tick;

                // Apply ship updates
                for update in ship_updates {
                    // Clone status if we need it for both player ship and sector ship
                    let status_clone = update.status.clone();

                    if let Some(ship) = self.ships.iter_mut().find(|s| s.id == update.id) {
                        if let Some(pos) = update.position {
                            ship.position = (pos.x, pos.y);
                        }
                        if let Some(hull) = update.hull_percent {
                            ship.hull_percent = hull;
                        }
                        if let Some(shield) = update.shield_percent {
                            ship.shield_percent = shield;
                        }
                        if let Some(status) = update.status {
                            ship.status = status;
                        }
                    }

                    // Also update our own ship if it matches
                    if Some(update.id) == self.ship_id {
                        if let Some(pos) = update.position {
                            self.position = (pos.x, pos.y);
                        }
                        if let Some(hull) = update.hull_percent {
                            self.ship_hull = hull;
                        }
                        if let Some(shield) = update.shield_percent {
                            self.ship_shields = shield;
                        }
                        if let Some(status) = status_clone {
                            self.ship_status = status;
                        }
                    }
                }

                // Add spawned ships
                for ship in ship_spawns {
                    self.ships.push(ShipInfo::from(ship));
                }

                // Remove despawned ships
                self.ships.retain(|s| !ship_despawns.contains(&s.id));

                // Update missions
                for update in mission_updates {
                    if let Some(mission) = self
                        .available_missions
                        .iter_mut()
                        .find(|m| m.id == update.id)
                    {
                        if let Some(status) = update.status {
                            mission.status = status;
                        }
                        if let Some(progress) = update.progress {
                            mission.progress = progress;
                        }
                    }
                }

                // Process events (could add to a log)
                for _event in events {
                    // TODO: Add to event log
                }
            }

            ServerMessage::ResourceUpdate {
                reputation,
                fame,
                ammunition,
                fuel,
                morale,
                experience,
            } => {
                self.reputation = reputation;
                self.fame = fame;
                self.ammunition = ammunition;
                self.fuel = fuel;
                self.morale = morale;
                self.experience = experience;
            }

            ServerMessage::MissionChoice {
                mission_id,
                description,
                choices,
            } => {
                self.mission_choice = Some(MissionChoice {
                    mission_id,
                    description,
                    choices: choices.into_iter().map(Choice::from).collect(),
                });
            }

            ServerMessage::MissionResult {
                mission_id: _,
                success: _,
                reputation_change: _,
                fame_change: _,
                narrative: _,
            } => {
                // Clear choice dialog
                self.mission_choice = None;
                // Mission updates will come via StateUpdate
            }

            ServerMessage::CombatUpdate {
                engagement_id,
                round,
                events,
                is_resolved,
                winner,
            } => {
                let log_entries: Vec<_> = events.into_iter().map(CombatLogEntry::from).collect();

                // Add to combat log
                for entry in &log_entries {
                    self.combat_log.push_back(entry.clone());
                    if self.combat_log.len() > 100 {
                        self.combat_log.pop_front();
                    }
                }

                // Update combat state
                if is_resolved {
                    self.combat = None;
                } else {
                    self.combat = Some(CombatState {
                        engagement_id,
                        round,
                        is_resolved,
                        winner,
                        events: log_entries,
                    });
                }
            }

            ServerMessage::ChatMessage {
                sender_id,
                sender_name,
                message,
                channel,
                timestamp,
            } => {
                self.chat_messages.push_back(ChatMessage {
                    sender_id,
                    sender_name,
                    message,
                    channel,
                    timestamp,
                });
                if self.chat_messages.len() > 100 {
                    self.chat_messages.pop_front();
                }
            }

            ServerMessage::Error { code, message } => {
                tracing::error!("Server error [{}]: {}", code, message);
            }

            ServerMessage::Pong {
                timestamp: _,
                server_tick,
            } => {
                self.server_tick = server_tick;
            }

            ServerMessage::Kicked { reason } => {
                tracing::warn!("Kicked from server: {}", reason);
                self.connected = false;
            }

            ServerMessage::SquadronUpdate { squadron, message: _ } => {
                self.squadron = squadron.map(SquadronInfo::from);
            }

            ServerMessage::SquadronInvite { .. } => {
                // TODO: Handle squadron invites
            }

            ServerMessage::AllianceProposal { .. } => {
                // TODO: Handle alliance proposals
            }

            ServerMessage::HailReceived { from_id: _, from_name } => {
                // Add hail to chat as system message
                self.chat_messages.push_back(ChatMessage {
                    sender_id: Uuid::nil(),
                    sender_name: "SYSTEM".into(),
                    message: format!("{} hails you!", from_name),
                    channel: ChatChannel::Sector,
                    timestamp: 0,
                });
            }

            ServerMessage::TickMetrics(metrics) => {
                self.tick_metrics = Some(metrics);
            }

            ServerMessage::TickMetricsHistory(history) => {
                self.tick_metrics_history = Some(history);
            }

            ServerMessage::Admin(_) => {
                // TODO: Handle admin messages
            }

            ServerMessage::Notification {
                message,
                notification_type: _,
            } => {
                // Add as system chat message for now
                self.chat_messages.push_back(ChatMessage {
                    sender_id: Uuid::nil(),
                    sender_name: "SYSTEM".into(),
                    message,
                    channel: ChatChannel::System,
                    timestamp: 0,
                });
            }

            ServerMessage::ChoiceRequired {
                choice_id: _,
                description: _,
                choices: _,
            } => {
                // TODO: Handle non-mission choices
            }

            ServerMessage::ScriptActionResult {
                action: _,
                success: _,
                error,
                data: _,
            } => {
                // Log errors from script actions
                if let Some(err) = error {
                    tracing::warn!("Script action error: {}", err);
                }
            }
        }
    }
}
