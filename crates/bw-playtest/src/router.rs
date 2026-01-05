//! Message routing for playtest mode
//!
//! Determines whether a player's messages should be routed to live state
//! or a playtest instance.

use std::sync::Arc;
use uuid::Uuid;

use crate::instance::PlaytestInstance;
use crate::manager::PlaytestManager;

/// Destination for a player's messages.
#[derive(Clone)]
pub enum MessageDestination {
    /// Route to live game state
    Live,
    /// Route to a playtest instance
    Playtest(Arc<PlaytestInstance>),
}

impl MessageDestination {
    /// Check if this is a playtest destination.
    pub fn is_playtest(&self) -> bool {
        matches!(self, Self::Playtest(_))
    }

    /// Get the playtest instance if this is a playtest destination.
    pub fn playtest(&self) -> Option<&Arc<PlaytestInstance>> {
        match self {
            Self::Playtest(instance) => Some(instance),
            Self::Live => None,
        }
    }

    /// Get the playtest ID if this is a playtest destination.
    pub fn playtest_id(&self) -> Option<Uuid> {
        match self {
            Self::Playtest(instance) => Some(instance.id),
            Self::Live => None,
        }
    }
}

impl PlaytestManager {
    /// Get the message routing destination for a player.
    ///
    /// Returns `MessageDestination::Live` if the player is not in any playtest,
    /// or `MessageDestination::Playtest(instance)` if they are.
    pub fn get_destination(&self, player_id: Uuid) -> MessageDestination {
        if let Some(instance) = self.get_for_player(player_id) {
            MessageDestination::Playtest(instance)
        } else {
            MessageDestination::Live
        }
    }
}
