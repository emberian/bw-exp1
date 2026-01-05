//! WebSocket message handling utilities

use bw_shared::ClientMessage;

/// Validate a client message.
///
/// Note: Most validation is now handled in action scripts for ScriptAction messages.
/// This function only validates infrastructure messages that stay in Rust.
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
        // ScriptAction validation is handled by action scripts
        _ => Ok(()),
    }
}
