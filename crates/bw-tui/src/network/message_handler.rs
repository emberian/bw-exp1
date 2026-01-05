//! Message handling utilities
//!
//! Most message handling is done directly in GameState::handle_server_message.
//! This module provides additional utilities for message processing.

#![allow(dead_code)]

use bw_shared::messages::ServerMessage;

/// Check if a message is a high-priority update that should be processed immediately
pub fn is_priority_message(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::CombatUpdate { .. }
            | ServerMessage::MissionChoice { .. }
            | ServerMessage::Error { .. }
            | ServerMessage::Kicked { .. }
    )
}

/// Check if a message should trigger a notification
pub fn should_notify(msg: &ServerMessage) -> Option<String> {
    match msg {
        ServerMessage::HailReceived { from_name, .. } => {
            Some(format!("{} hails you!", from_name))
        }
        ServerMessage::SquadronInvite {
            squadron_name,
            inviter_name,
            ..
        } => Some(format!("{} invited you to {}", inviter_name, squadron_name)),
        ServerMessage::MissionResult {
            success, narrative, ..
        } => {
            if *success {
                Some(format!("Mission Complete: {}", narrative))
            } else {
                Some(format!("Mission Failed: {}", narrative))
            }
        }
        ServerMessage::Notification { message, .. } => Some(message.clone()),
        _ => None,
    }
}
