//! Message API bindings for Rhai
//!
//! Allows scripts to send notifications and choices to players.
//! Messages are queued as mutations and sent after script execution.

use rhai::{Engine, Array, Map};
use uuid::Uuid;

use crate::state::ChoiceOption;
use super::state_api::with_accessor;

/// Register message API functions with the engine.
pub fn register(engine: &mut Engine) {
    // send_notification(player_id: String, message: String) -> bool
    // Sends a simple notification to a player
    engine.register_fn("send_notification", |player_id: String, message: String| -> bool {
        send_notification_impl(player_id, message, "info".to_string())
    });

    // send_notification_type(player_id: String, message: String, notification_type: String) -> bool
    // Sends a typed notification (info, warning, error, success)
    engine.register_fn("send_notification_type", |player_id: String, message: String, notification_type: String| -> bool {
        send_notification_impl(player_id, message, notification_type)
    });

    // send_warning(player_id: String, message: String) -> bool
    engine.register_fn("send_warning", |player_id: String, message: String| -> bool {
        send_notification_impl(player_id, message, "warning".to_string())
    });

    // send_error(player_id: String, message: String) -> bool
    engine.register_fn("send_error", |player_id: String, message: String| -> bool {
        send_notification_impl(player_id, message, "error".to_string())
    });

    // send_success(player_id: String, message: String) -> bool
    engine.register_fn("send_success", |player_id: String, message: String| -> bool {
        send_notification_impl(player_id, message, "success".to_string())
    });

    // send_choice(player_id: String, choice_id: String, description: String, choices: Array) -> bool
    // Sends a choice dialog to a player
    // choices: Array of maps with keys: id, text, is_available (optional), requirement_text (optional)
    engine.register_fn("send_choice", |player_id: String, choice_id: String, description: String, choices: Array| -> bool {
        let id = match Uuid::parse_str(&player_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        // Parse choices array
        let parsed_choices: Vec<ChoiceOption> = choices.into_iter()
            .filter_map(|item| {
                let map = item.try_cast::<Map>()?;
                let option_id = map.get("id")?.clone().into_string().ok()?;
                let text = map.get("text")?.clone().into_string().ok()?;
                let is_available = map.get("is_available")
                    .and_then(|v| v.as_bool().ok())
                    .unwrap_or(true);
                let requirement_text = map.get("requirement_text")
                    .and_then(|v| v.clone().into_string().ok());

                Some(ChoiceOption {
                    id: option_id,
                    text,
                    is_available,
                    requirement_text,
                })
            })
            .collect();

        if parsed_choices.is_empty() {
            return false;
        }

        with_accessor(|accessor| -> bool {
            accessor.send_choice(id, choice_id.clone(), description.clone(), parsed_choices.clone()).is_ok()
        }).unwrap_or(false)
    });

    // broadcast_to_sector(sector_id: String, message: String) -> bool
    // Broadcasts a message to all players in a sector
    engine.register_fn("broadcast_to_sector", |sector_id: String, message: String| -> bool {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        with_accessor(|accessor| -> bool {
            accessor.broadcast_to_sector(id, message.clone(), "info".to_string()).is_ok()
        }).unwrap_or(false)
    });

    // broadcast_warning_to_sector(sector_id: String, message: String) -> bool
    engine.register_fn("broadcast_warning_to_sector", |sector_id: String, message: String| -> bool {
        let id = match Uuid::parse_str(&sector_id) {
            Ok(id) => id,
            Err(_) => return false,
        };

        with_accessor(|accessor| -> bool {
            accessor.broadcast_to_sector(id, message.clone(), "warning".to_string()).is_ok()
        }).unwrap_or(false)
    });
}

fn send_notification_impl(player_id: String, message: String, notification_type: String) -> bool {
    let id = match Uuid::parse_str(&player_id) {
        Ok(id) => id,
        Err(_) => return false,
    };

    with_accessor(|accessor| -> bool {
        accessor.send_notification(id, message.clone(), notification_type.clone()).is_ok()
    }).unwrap_or(false)
}
