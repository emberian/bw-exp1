//! WebSocket message handling utilities

use bw_shared::ClientMessage;

/// Validate a client message.
pub fn validate_message(msg: &ClientMessage) -> Result<(), &'static str> {
    match msg {
        ClientMessage::SendChat { message, .. } => {
            if message.len() > 500 {
                return Err("Message too long");
            }
            if message.is_empty() {
                return Err("Message empty");
            }
            Ok(())
        }
        ClientMessage::CreateSquadron { name, tag } => {
            if name.len() < 3 || name.len() > 32 {
                return Err("Squadron name must be 3-32 characters");
            }
            if tag.len() < 2 || tag.len() > 5 {
                return Err("Squadron tag must be 2-5 characters");
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
