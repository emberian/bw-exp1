//! Game state management
//!
//! Reactive signals for all game data.

use leptos::prelude::*;
use uuid::Uuid;

/// Global game state, provided at app root.
#[derive(Clone)]
pub struct GameState {
    // Connection state
    pub connected: RwSignal<bool>,
    pub player_id: RwSignal<Option<Uuid>>,
    pub username: RwSignal<String>,

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
    pub ship_hull: RwSignal<f32>,
    pub ship_shields: RwSignal<f32>,
    pub ship_status: RwSignal<String>,

    // Position
    pub position_x: RwSignal<f64>,
    pub position_y: RwSignal<f64>,

    // Current sector
    pub sector_name: RwSignal<String>,
    pub sector_danger: RwSignal<String>,

    // Squadron state
    pub squadron: RwSignal<Option<SquadronInfo>>,

    // UI state
    pub selected_target: RwSignal<Option<Uuid>>,
    pub active_mission: RwSignal<Option<Uuid>>,
    pub show_squadron_dialog: RwSignal<bool>,
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

impl GameState {
    pub fn new() -> Self {
        Self {
            connected: RwSignal::new(false),
            player_id: RwSignal::new(None),
            username: RwSignal::new("Officer".to_string()),

            reputation: RwSignal::new(100),
            fame: RwSignal::new(0),

            ammunition: RwSignal::new(100.0),
            fuel: RwSignal::new(100.0),

            morale: RwSignal::new(50.0),
            experience: RwSignal::new(0),

            ship_hull: RwSignal::new(100.0),
            ship_shields: RwSignal::new(50.0),
            ship_status: RwSignal::new("Idle".to_string()),

            position_x: RwSignal::new(0.0),
            position_y: RwSignal::new(0.0),

            sector_name: RwSignal::new("Thornwick Sector".to_string()),
            sector_danger: RwSignal::new("Moderate".to_string()),

            squadron: RwSignal::new(None),

            selected_target: RwSignal::new(None),
            active_mission: RwSignal::new(None),
            show_squadron_dialog: RwSignal::new(false),
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

    /// Check if player is in a squadron.
    pub fn in_squadron(&self) -> bool {
        self.squadron.get().is_some()
    }

    /// Check if player can manage squadron (leader or officer).
    pub fn can_manage_squadron(&self) -> bool {
        self.squadron.get().map(|s| s.is_leader || s.is_officer).unwrap_or(false)
    }
}
