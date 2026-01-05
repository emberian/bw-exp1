//! PlayerView facade for scripts
//!
//! Provides fluent property access for players:
//! ```rhai
//! if !player.spend_credits(1000) {
//!     player.notify("Insufficient credits!");
//! }
//! player.ship.add_cargo("fuel", 10, 50);
//! ```

use rhai::{Dynamic, Engine, Array, CustomType, TypeBuilder};
use uuid::Uuid;

use crate::context::with_accessor;
use bw_game::state::PlayerChanges;
use super::ShipView;

/// A facade view of a player for scripts.
///
/// Provides natural property access and safe mutation methods.
#[derive(Debug, Clone)]
pub struct PlayerView {
    pub(crate) id: Uuid,
}

impl PlayerView {
    /// Create a new player view.
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }

    /// Create from a string UUID.
    pub fn from_string(id: &str) -> Option<Self> {
        Uuid::parse_str(id).ok().map(Self::new)
    }

    // =========================================================================
    // Getters - read from snapshot via accessor
    // =========================================================================

    /// Get player ID as string.
    pub fn get_id(&mut self) -> String {
        self.id.to_string()
    }

    /// Get username.
    pub fn get_username(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.username.clone())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get reputation.
    pub fn get_reputation(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.reputation as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get fame.
    pub fn get_fame(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.fame as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get credits.
    pub fn get_credits(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.credits)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get game mode.
    pub fn get_game_mode(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.game_mode.clone())
                .unwrap_or_else(|| "standard".to_string())
        }).unwrap_or_else(|| "standard".to_string())
    }

    /// Get active ship ID.
    pub fn get_active_ship_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.active_ship_id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get active ship as a ShipView.
    pub fn get_ship(&mut self) -> Dynamic {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| Dynamic::from(ShipView::new(p.active_ship_id)))
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    }

    /// Get sector ID.
    pub fn get_sector_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.sector_id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get faction ID.
    pub fn get_faction_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.faction_id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get squadron ID (empty if not in a squadron).
    pub fn get_squadron_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .and_then(|p| p.squadron_id)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Check if player is online.
    pub fn get_is_online(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.is_online)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Get missions completed count.
    pub fn get_missions_completed(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.missions_completed as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Get missions failed count.
    pub fn get_missions_failed(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.missions_failed as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    /// Check if player is disgraced (reputation < -500).
    pub fn get_is_disgraced(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| p.is_disgraced)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Get owned ship IDs.
    pub fn get_owned_ships(&mut self) -> Array {
        with_accessor(|accessor| {
            accessor.get_player(self.id)
                .ok()
                .flatten()
                .map(|p| {
                    p.owned_ships.iter()
                        .map(|id| Dynamic::from(id.to_string()))
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    // =========================================================================
    // Setters - queue mutations
    // =========================================================================

    /// Set credits directly.
    pub fn set_credits(&mut self, value: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                credits: Some(value.max(0)),
                ..Default::default()
            });
        });
    }

    /// Set reputation directly.
    pub fn set_reputation(&mut self, value: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                reputation: Some(value as i32),
                ..Default::default()
            });
        });
    }

    /// Set fame directly.
    pub fn set_fame(&mut self, value: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                fame: Some(value.max(0) as i32),
                ..Default::default()
            });
        });
    }

    // =========================================================================
    // Methods - safe operations
    // =========================================================================

    /// Add credits to the player.
    pub fn add_credits(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                credits_delta: Some(amount),
                ..Default::default()
            });
        });
    }

    /// Spend credits (returns true if successful, false if insufficient).
    pub fn spend_credits(&mut self, amount: i64) -> bool {
        if amount <= 0 {
            return true;
        }
        let current = self.get_credits();
        if current < amount {
            return false;
        }
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                credits_delta: Some(-amount),
                ..Default::default()
            });
        });
        true
    }

    /// Adjust reputation (positive or negative delta).
    pub fn adjust_reputation(&mut self, delta: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                reputation_delta: Some(delta as i32),
                ..Default::default()
            });
        });
    }

    /// Adjust fame (positive or negative delta).
    pub fn adjust_fame(&mut self, delta: i64) {
        with_accessor(|accessor| {
            let _ = accessor.modify_player(self.id, PlayerChanges {
                fame_delta: Some(delta as i32),
                ..Default::default()
            });
        });
    }

    /// Send a notification to this player.
    pub fn notify(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.send_notification(self.id, message, "info".to_string());
        });
    }

    /// Send a warning notification.
    pub fn warn(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.send_notification(self.id, message, "warning".to_string());
        });
    }

    /// Send an error notification.
    pub fn error(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.send_notification(self.id, message, "error".to_string());
        });
    }

    /// Send a success notification.
    pub fn success(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.send_notification(self.id, message, "success".to_string());
        });
    }

    /// Check if player exists.
    pub fn exists(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_player(self.id).ok().flatten().is_some()
        }).unwrap_or(false)
    }

    /// Check if player can afford an amount.
    pub fn can_afford(&mut self, amount: i64) -> bool {
        self.get_credits() >= amount
    }

    /// Check if player is in a squadron.
    pub fn in_squadron(&mut self) -> bool {
        !self.get_squadron_id().is_empty()
    }
}

// Implement CustomType for automatic registration
impl CustomType for PlayerView {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("PlayerView")
            .with_get("id", Self::get_id)
            .with_get("username", Self::get_username)
            .with_get_set("reputation", Self::get_reputation, Self::set_reputation)
            .with_get_set("fame", Self::get_fame, Self::set_fame)
            .with_get_set("credits", Self::get_credits, Self::set_credits)
            .with_get("game_mode", Self::get_game_mode)
            .with_get("active_ship_id", Self::get_active_ship_id)
            .with_get("ship", Self::get_ship)
            .with_get("sector_id", Self::get_sector_id)
            .with_get("faction_id", Self::get_faction_id)
            .with_get("squadron_id", Self::get_squadron_id)
            .with_get("is_online", Self::get_is_online)
            .with_get("missions_completed", Self::get_missions_completed)
            .with_get("missions_failed", Self::get_missions_failed)
            .with_get("is_disgraced", Self::get_is_disgraced)
            .with_get("owned_ships", Self::get_owned_ships);
    }
}

/// Register PlayerView with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register the type with automatic getters/setters
    engine.build_type::<PlayerView>();

    // Register methods
    engine.register_fn("add_credits", PlayerView::add_credits);
    engine.register_fn("spend_credits", PlayerView::spend_credits);
    engine.register_fn("adjust_reputation", PlayerView::adjust_reputation);
    engine.register_fn("adjust_fame", PlayerView::adjust_fame);
    engine.register_fn("notify", PlayerView::notify);
    engine.register_fn("warn", PlayerView::warn);
    engine.register_fn("error", PlayerView::error);
    engine.register_fn("success", PlayerView::success);
    engine.register_fn("exists", PlayerView::exists);
    engine.register_fn("can_afford", PlayerView::can_afford);
    engine.register_fn("in_squadron", PlayerView::in_squadron);

    // Constructor from string ID
    engine.register_fn("player", |id: String| -> Dynamic {
        match PlayerView::from_string(&id) {
            Some(view) => Dynamic::from(view),
            None => Dynamic::UNIT,
        }
    });
}
