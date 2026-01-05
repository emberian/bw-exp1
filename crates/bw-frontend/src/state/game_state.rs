//! Game state management
//!
//! Reactive signals for all game data.

use leptos::prelude::*;
use uuid::Uuid;

use bw_shared::dto::*;
use bw_shared::ChatChannel;

/// Global game state, provided at app root.
#[derive(Clone, Copy)]
pub struct GameState {
    // Connection state
    pub connected: RwSignal<bool>,
    pub player_id: RwSignal<Option<Uuid>>,
    pub username: RwSignal<String>,
    pub server_tick: RwSignal<u64>,

    // Player resources
    pub reputation: RwSignal<i32>,
    pub fame: RwSignal<i32>,

    // Ship resources
    pub ammunition: RwSignal<f32>,
    pub fuel: RwSignal<f32>,

    // Crew resources
    pub morale: RwSignal<f32>,
    pub experience: RwSignal<i32>,

    // Ship state
    pub ship_id: RwSignal<Option<Uuid>>,
    pub ship_name: RwSignal<String>,
    pub ship_class: RwSignal<String>,
    pub ship_hull: RwSignal<f32>,
    pub ship_shields: RwSignal<f32>,
    pub ship_max_hull: RwSignal<f32>,
    pub ship_max_shields: RwSignal<f32>,
    pub ship_status: RwSignal<String>,
    pub ship_weapons: RwSignal<Vec<WeaponInfo>>,

    // Position
    pub position_x: RwSignal<f64>,
    pub position_y: RwSignal<f64>,

    // Docked state
    pub docked_station_id: RwSignal<Option<Uuid>>,
    pub docked_station_name: RwSignal<String>,
    pub docked_station_services: RwSignal<Vec<String>>,

    // Current sector
    pub sector_id: RwSignal<Option<Uuid>>,
    pub sector_name: RwSignal<String>,
    pub sector_danger: RwSignal<String>,

    // Adjacent sectors
    pub adjacent_sectors: RwSignal<Vec<AdjacentSectorInfo>>,

    // Locations in sector
    pub locations: RwSignal<Vec<LocationInfo>>,

    // Other ships in sector
    pub ships: RwSignal<Vec<ShipInfo>>,

    // Missions
    pub available_missions: RwSignal<Vec<MissionInfo>>,
    pub active_mission: RwSignal<Option<MissionInfo>>,

    // Mission choice dialog
    pub mission_choice: RwSignal<Option<MissionChoiceInfo>>,

    // Chat messages
    pub chat_messages: RwSignal<Vec<ChatMessageInfo>>,

    // Squadron state
    pub squadron: RwSignal<Option<SquadronInfo>>,

    // UI state
    pub selected_target: RwSignal<Option<Uuid>>,
    pub show_squadron_dialog: RwSignal<bool>,
    pub show_mission_dialog: RwSignal<bool>,

    // Pending invitations/proposals
    pub pending_squadron_invites: RwSignal<Vec<SquadronInviteInfo>>,
    pub pending_alliance_proposals: RwSignal<Vec<AllianceProposalInfo>>,

    // Combat state
    pub combat_engagement_id: RwSignal<Option<Uuid>>,
    pub combat_round: RwSignal<u32>,
    pub combat_events: RwSignal<Vec<CombatEventInfo>>,
    pub combat_resolved: RwSignal<bool>,
    pub combat_winner: RwSignal<Option<String>>,

    // Error/notification
    pub last_error: RwSignal<Option<String>>,
    pub notification: RwSignal<Option<String>>,
}

/// Location info for display.
#[derive(Clone, Debug, Default)]
pub struct LocationInfo {
    pub id: Uuid,
    pub name: String,
    pub location_type: String,
    pub x: f64,
    pub y: f64,
    pub services: Vec<String>,
}

/// Ship info for display.
#[derive(Clone, Debug, Default)]
pub struct ShipInfo {
    pub id: Uuid,
    pub name: String,
    pub ship_class: String,
    pub x: f64,
    pub y: f64,
    pub hull_percent: f32,
    pub is_player: bool,
    pub is_hostile: bool,
    pub status: String,
}

/// Mission info for display.
#[derive(Clone, Debug, Default)]
pub struct MissionInfo {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub mission_type: String,
    pub status: String,
    pub reputation_reward: i32,
    pub fame_reward: i32,
    pub expires_in_seconds: Option<u32>,
    pub progress: f32,
    pub can_accept: bool,
    pub is_high_profile: bool,
}

/// Mission choice info for dialog.
#[derive(Clone, Debug, Default)]
pub struct MissionChoiceInfo {
    pub mission_id: Uuid,
    pub description: String,
    pub choices: Vec<ChoiceInfo>,
}

/// Single choice option.
#[derive(Clone, Debug, Default)]
pub struct ChoiceInfo {
    pub id: String,
    pub text: String,
    pub is_available: bool,
    pub requirement_text: Option<String>,
}

/// Chat message for display.
#[derive(Clone, Debug)]
pub struct ChatMessageInfo {
    pub id: u64,
    pub sender_name: String,
    pub message: String,
    pub channel: ChatChannel,
    pub timestamp: u64,
    pub is_system: bool,
}

/// Squadron information for display.
#[derive(Clone, Debug, Default)]
pub struct SquadronInfo {
    pub id: Uuid,
    pub name: String,
    pub tag: String,
    pub motto: Option<String>,
    pub leader_name: String,
    pub member_count: u32,
    pub reputation_bonus: f32,
    pub fame_bonus: f32,
    pub is_at_war: bool,
    pub is_leader: bool,
    pub is_officer: bool,
}

/// Combat event info for display.
#[derive(Clone, Debug)]
pub struct CombatEventInfo {
    pub event_type: String,
    pub attacker_name: String,
    pub target_name: String,
    pub damage: Option<f32>,
    pub hit: bool,
    pub message: String,
}

/// Weapon info for display.
#[derive(Clone, Debug, Default)]
pub struct WeaponInfo {
    pub name: String,
    pub weapon_type: String,
    pub damage: f32,
    pub accuracy: f32,
    pub ammo_cost: f32,
}

/// Adjacent sector info for display.
#[derive(Clone, Debug, Default)]
pub struct AdjacentSectorInfo {
    pub id: Uuid,
    pub name: String,
    pub danger_level: String,
}

/// Squadron invite info for display.
#[derive(Clone, Debug)]
pub struct SquadronInviteInfo {
    pub invite_id: Uuid,
    pub squadron_id: Uuid,
    pub squadron_name: String,
    pub squadron_tag: String,
    pub inviter_name: String,
}

/// Alliance proposal info for display.
#[derive(Clone, Debug)]
pub struct AllianceProposalInfo {
    pub proposal_id: Uuid,
    pub from_squadron_id: Uuid,
    pub from_squadron_name: String,
    pub from_squadron_tag: String,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    pub fn new() -> Self {
        Self {
            connected: RwSignal::new(false),
            player_id: RwSignal::new(None),
            username: RwSignal::new("Officer".to_string()),
            server_tick: RwSignal::new(0),

            reputation: RwSignal::new(100),
            fame: RwSignal::new(0),

            ammunition: RwSignal::new(100.0),
            fuel: RwSignal::new(100.0),

            morale: RwSignal::new(50.0),
            experience: RwSignal::new(0),

            ship_id: RwSignal::new(None),
            ship_name: RwSignal::new("Unknown Ship".to_string()),
            ship_class: RwSignal::new("Unknown".to_string()),
            ship_hull: RwSignal::new(100.0),
            ship_shields: RwSignal::new(50.0),
            ship_max_hull: RwSignal::new(100.0),
            ship_max_shields: RwSignal::new(50.0),
            ship_status: RwSignal::new("Idle".to_string()),
            ship_weapons: RwSignal::new(vec![]),

            position_x: RwSignal::new(0.0),
            position_y: RwSignal::new(0.0),

            docked_station_id: RwSignal::new(None),
            docked_station_name: RwSignal::new(String::new()),
            docked_station_services: RwSignal::new(vec![]),

            sector_id: RwSignal::new(None),
            sector_name: RwSignal::new("Unknown Sector".to_string()),
            sector_danger: RwSignal::new("Unknown".to_string()),

            adjacent_sectors: RwSignal::new(vec![]),

            locations: RwSignal::new(vec![]),
            ships: RwSignal::new(vec![]),

            available_missions: RwSignal::new(vec![]),
            active_mission: RwSignal::new(None),
            mission_choice: RwSignal::new(None),

            chat_messages: RwSignal::new(vec![]),

            squadron: RwSignal::new(None),

            selected_target: RwSignal::new(None),
            show_squadron_dialog: RwSignal::new(false),
            show_mission_dialog: RwSignal::new(false),

            pending_squadron_invites: RwSignal::new(vec![]),
            pending_alliance_proposals: RwSignal::new(vec![]),

            combat_engagement_id: RwSignal::new(None),
            combat_round: RwSignal::new(0),
            combat_events: RwSignal::new(vec![]),
            combat_resolved: RwSignal::new(false),
            combat_winner: RwSignal::new(None),

            last_error: RwSignal::new(None),
            notification: RwSignal::new(None),
        }
    }

    /// Update from server resource message.
    pub fn update_resources(
        &self,
        reputation: i32,
        fame: i32,
        ammunition: f32,
        fuel: f32,
        morale: f32,
        experience: i32,
    ) {
        self.reputation.set(reputation);
        self.fame.set(fame);
        self.ammunition.set(ammunition);
        self.fuel.set(fuel);
        self.morale.set(morale);
        self.experience.set(experience);
    }

    /// Update ship position.
    pub fn update_position(&self, x: f64, y: f64) {
        self.position_x.set(x);
        self.position_y.set(y);
    }

    /// Update ship status.
    pub fn update_ship(&self, hull: f32, shields: f32, status: String) {
        self.ship_hull.set(hull);
        self.ship_shields.set(shields);
        self.ship_status.set(status);
    }

    /// Update squadron info.
    pub fn update_squadron(&self, info: Option<SquadronInfo>) {
        self.squadron.set(info);
    }

    /// Handle initial state from server.
    pub fn handle_initial_state(
        &self,
        player: PlayerDto,
        ship: ShipDto,
        sector: SectorDto,
        ships: Vec<ShipDto>,
        missions: Vec<MissionDto>,
    ) {
        // Player info
        self.player_id.set(Some(player.id));
        self.username.set(player.username);
        self.reputation.set(player.reputation);
        self.fame.set(player.fame);

        // Ship info
        self.ship_id.set(Some(ship.id));
        self.ship_name.set(ship.name.clone());
        self.ship_class.set(ship.ship_class.clone());
        self.position_x.set(ship.position.x);
        self.position_y.set(ship.position.y);
        self.ship_hull.set(ship.hull_percent);
        self.ship_shields.set(ship.shield_percent);
        self.ship_status.set(ship.status.clone());

        // Update docked state based on ship status
        if ship.status == "Docked" {
            // Will be populated when station info is available
        } else {
            self.docked_station_id.set(None);
            self.docked_station_name.set(String::new());
            self.docked_station_services.set(vec![]);
        }

        // Sector info
        self.sector_id.set(Some(sector.id));
        self.sector_name.set(sector.name.clone());
        self.sector_danger.set(sector.danger_level);

        // Adjacent sectors
        let adjacent: Vec<AdjacentSectorInfo> = sector.adjacent_sectors.into_iter().map(|s| AdjacentSectorInfo {
            id: s.id,
            name: s.name,
            danger_level: s.danger_level,
        }).collect();
        self.adjacent_sectors.set(adjacent);

        // Locations
        let locs: Vec<LocationInfo> = sector.locations.into_iter().map(|l| LocationInfo {
            id: l.id,
            name: l.name,
            location_type: l.location_type,
            x: l.position.x,
            y: l.position.y,
            services: l.services,
        }).collect();
        self.locations.set(locs);

        // Other ships
        let ship_infos: Vec<ShipInfo> = ships.into_iter().map(|s| ShipInfo {
            id: s.id,
            name: s.name,
            ship_class: s.ship_class,
            x: s.position.x,
            y: s.position.y,
            hull_percent: s.hull_percent,
            is_player: s.is_player,
            is_hostile: s.is_hostile,
            status: s.status,
        }).collect();
        self.ships.set(ship_infos);

        // Missions
        let mission_infos: Vec<MissionInfo> = missions.into_iter().map(|m| MissionInfo {
            id: m.id,
            title: m.title,
            description: m.description,
            mission_type: m.mission_type,
            status: m.status,
            reputation_reward: m.reputation_reward,
            fame_reward: m.fame_reward,
            expires_in_seconds: m.expires_in_seconds,
            progress: m.progress,
            can_accept: m.can_accept,
            is_high_profile: m.is_high_profile,
        }).collect();
        self.available_missions.set(mission_infos);
    }

    /// Handle state update from server.
    pub fn handle_state_update(
        &self,
        tick: u64,
        ship_updates: Vec<ShipUpdateDto>,
        ship_spawns: Vec<ShipDto>,
        ship_despawns: Vec<Uuid>,
        mission_updates: Vec<MissionUpdateDto>,
        events: Vec<GameEventDto>,
    ) {
        self.server_tick.set(tick);

        // Update existing ships
        self.ships.update(|ships| {
            for update in ship_updates {
                // Check if it's our ship
                if Some(update.id) == self.ship_id.get_untracked() {
                    if let Some(pos) = &update.position {
                        self.position_x.set(pos.x);
                        self.position_y.set(pos.y);
                    }
                    if let Some(hull) = update.hull_percent {
                        self.ship_hull.set(hull);
                    }
                    if let Some(shields) = update.shield_percent {
                        self.ship_shields.set(shields);
                    }
                    if let Some(status) = update.status.clone() {
                        // Clear docked state if no longer docked
                        if status != "Docked" {
                            self.docked_station_id.set(None);
                            self.docked_station_name.set(String::new());
                            self.docked_station_services.set(vec![]);
                        }
                        self.ship_status.set(status);
                    }
                }

                // Update in ships list
                if let Some(ship) = ships.iter_mut().find(|s| s.id == update.id) {
                    if let Some(pos) = update.position {
                        ship.x = pos.x;
                        ship.y = pos.y;
                    }
                    if let Some(hull) = update.hull_percent {
                        ship.hull_percent = hull;
                    }
                    if let Some(status) = update.status {
                        ship.status = status;
                    }
                }
            }

            // Remove despawned ships
            ships.retain(|s| !ship_despawns.contains(&s.id));

            // Add spawned ships
            for spawn in ship_spawns {
                ships.push(ShipInfo {
                    id: spawn.id,
                    name: spawn.name,
                    ship_class: spawn.ship_class,
                    x: spawn.position.x,
                    y: spawn.position.y,
                    hull_percent: spawn.hull_percent,
                    is_player: spawn.is_player,
                    is_hostile: spawn.is_hostile,
                    status: spawn.status,
                });
            }
        });

        // Process mission updates
        for update in mission_updates {
            // Clone status for use in both closures
            let status_for_available = update.status.clone();
            let status_for_active = update.status.clone();
            let update_id = update.id;
            let update_progress = update.progress;

            self.available_missions.update(|missions| {
                if let Some(mission) = missions.iter_mut().find(|m| m.id == update_id) {
                    if let Some(status) = status_for_available {
                        mission.status = status;
                    }
                    if let Some(progress) = update_progress {
                        mission.progress = progress;
                    }
                }
            });

            // Also update active mission if it matches
            self.active_mission.update(|active| {
                if let Some(mission) = active {
                    if mission.id == update_id {
                        if let Some(status) = status_for_active {
                            mission.status = status;
                        }
                        if let Some(progress) = update_progress {
                            mission.progress = progress;
                        }
                    }
                }
            });
        }

        // Process game events - show them as notifications
        for event in events {
            // Create a human-readable notification from the event
            let message = format_event_message(&event);
            if !message.is_empty() {
                self.notification.set(Some(message));
            }
        }
    }

    /// Handle mission choice from server.
    pub fn handle_mission_choice(&self, mission_id: Uuid, description: String, choices: Vec<ChoiceDto>) {
        let choice_infos: Vec<ChoiceInfo> = choices.into_iter().map(|c| ChoiceInfo {
            id: c.id,
            text: c.text,
            is_available: c.is_available,
            requirement_text: c.requirement_text,
        }).collect();

        self.mission_choice.set(Some(MissionChoiceInfo {
            mission_id,
            description,
            choices: choice_infos,
        }));
        self.show_mission_dialog.set(true);
    }

    /// Handle mission result from server.
    pub fn handle_mission_result(
        &self,
        _mission_id: Uuid,
        success: bool,
        reputation_change: i32,
        fame_change: i32,
        narrative: String,
    ) {
        // Clear mission choice dialog
        self.mission_choice.set(None);
        self.show_mission_dialog.set(false);

        // Show result as notification
        let result_text = if success { "Mission Complete!" } else { "Mission Failed" };
        let notification = format!("{} {} (Rep: {:+}, Fame: {:+})",
            result_text, narrative, reputation_change, fame_change);
        self.notification.set(Some(notification));

        // Clear active mission
        self.active_mission.set(None);
    }

    /// Handle combat update from server.
    pub fn handle_combat_update(
        &self,
        engagement_id: Uuid,
        round: u32,
        events: Vec<bw_shared::dto::CombatEventDto>,
        is_resolved: bool,
        winner: Option<String>,
    ) {
        self.combat_engagement_id.set(Some(engagement_id));
        self.combat_round.set(round);
        self.combat_resolved.set(is_resolved);
        self.combat_winner.set(winner.clone());

        // Convert DTOs to display info
        let event_infos: Vec<CombatEventInfo> = events.into_iter().map(|e| CombatEventInfo {
            event_type: e.event_type,
            attacker_name: e.attacker_name,
            target_name: e.target_name,
            damage: e.damage,
            hit: e.hit,
            message: e.message,
        }).collect();

        // Append new events to existing list
        self.combat_events.update(|existing| {
            existing.extend(event_infos);
            // Keep last 50 events
            if existing.len() > 50 {
                existing.drain(0..existing.len() - 50);
            }
        });

        // If combat resolved, show notification
        if is_resolved {
            if let Some(w) = winner {
                self.notification.set(Some(format!("Combat resolved. Winner: {}", w)));
            } else {
                self.notification.set(Some("Combat resolved.".to_string()));
            }
        }
    }

    /// Clear combat state.
    pub fn clear_combat(&self) {
        self.combat_engagement_id.set(None);
        self.combat_round.set(0);
        self.combat_events.set(vec![]);
        self.combat_resolved.set(false);
        self.combat_winner.set(None);
    }

    /// Check if in active combat.
    pub fn in_combat(&self) -> bool {
        self.combat_engagement_id.get().is_some() && !self.combat_resolved.get()
    }

    /// Add a chat message.
    pub fn add_chat_message(&self, sender_name: String, message: String, channel: ChatChannel, timestamp: u64) {
        self.chat_messages.update(|msgs| {
            let id = msgs.len() as u64;
            let is_system = channel == ChatChannel::System;
            msgs.push(ChatMessageInfo {
                id,
                sender_name,
                message,
                channel,
                timestamp,
                is_system,
            });
            // Keep last 100 messages
            if msgs.len() > 100 {
                msgs.remove(0);
            }
        });
    }

    /// Set error message.
    pub fn set_error(&self, message: String) {
        self.last_error.set(Some(message));
    }

    /// Clear error message.
    pub fn clear_error(&self) {
        self.last_error.set(None);
    }

    /// Check if player is in a squadron.
    pub fn in_squadron(&self) -> bool {
        self.squadron.get().is_some()
    }

    /// Check if player can manage squadron (leader or officer).
    pub fn can_manage_squadron(&self) -> bool {
        self.squadron.get().map(|s| s.is_leader || s.is_officer).unwrap_or(false)
    }
}

/// Format a game event into a human-readable notification message.
fn format_event_message(event: &GameEventDto) -> String {
    let event_type = event.event_type.as_str();

    match event_type {
        "ship_destroyed" => {
            if let (Some(actor), Some(target)) = (&event.actor_name, &event.target_name) {
                format!("{} destroyed {}!", actor, target)
            } else {
                event.message.clone()
            }
        }
        "player_joined" => {
            if let Some(actor) = &event.actor_name {
                format!("{} has entered the sector", actor)
            } else {
                event.message.clone()
            }
        }
        "player_left" => {
            if let Some(actor) = &event.actor_name {
                format!("{} has left the sector", actor)
            } else {
                event.message.clone()
            }
        }
        "mission_completed" => {
            if let Some(actor) = &event.actor_name {
                format!("{} completed a mission!", actor)
            } else {
                "Mission completed!".to_string()
            }
        }
        "combat_started" => {
            if let (Some(actor), Some(target)) = (&event.actor_name, &event.target_name) {
                format!("{} engaged {}!", actor, target)
            } else {
                "Combat has started!".to_string()
            }
        }
        "distress_signal" => {
            "Distress signal detected nearby!".to_string()
        }
        "sera_incursion" => {
            "WARNING: Sera incursion detected!".to_string()
        }
        "drone_swarm" => {
            "WARNING: Drone swarm approaching!".to_string()
        }
        _ => event.message.clone(),
    }
}
