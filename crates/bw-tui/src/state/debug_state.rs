//! Debug and GM state
//!
//! Contains debug state including message logs and metrics.

#![allow(dead_code)]

use std::collections::VecDeque;

use bw_shared::messages::ServerMessage;
use chrono::{DateTime, Utc};

/// Direction of a logged message
#[derive(Debug, Clone, Copy)]
pub enum MessageDirection {
    Sent,
    Received,
}

/// A logged raw message
#[derive(Debug, Clone)]
pub struct RawMessage {
    pub direction: MessageDirection,
    pub timestamp: DateTime<Utc>,
    pub message_type: String,
    pub summary: String,
}

/// Debug/GM state
#[derive(Debug)]
pub struct DebugState {
    /// Show debug panel
    pub show_debug: bool,
    /// Show raw message log
    pub show_messages: bool,
    /// Raw message history (newest first)
    pub message_log: VecDeque<RawMessage>,
    /// Message log capacity
    pub message_log_capacity: usize,
    /// Show tick metrics
    pub show_metrics: bool,
    /// Show state inspector
    pub show_inspector: bool,
}

impl DebugState {
    pub fn new(enabled: bool) -> Self {
        Self {
            show_debug: enabled,
            show_messages: enabled,
            show_metrics: enabled,
            show_inspector: false,
            message_log: VecDeque::new(),
            message_log_capacity: 100,
        }
    }

    /// Log a message
    pub fn log_message(&mut self, sent: bool, msg: &ServerMessage) {
        let (message_type, summary) = Self::describe_message(msg);
        self.message_log.push_front(RawMessage {
            direction: if sent {
                MessageDirection::Sent
            } else {
                MessageDirection::Received
            },
            timestamp: Utc::now(),
            message_type,
            summary,
        });

        // Trim to capacity
        while self.message_log.len() > self.message_log_capacity {
            self.message_log.pop_back();
        }
    }

    /// Get a description of a server message
    fn describe_message(msg: &ServerMessage) -> (String, String) {
        match msg {
            ServerMessage::AuthResult { success, .. } => {
                ("AuthResult".into(), format!("success={}", success))
            }
            ServerMessage::InitialState {
                sector, ships, missions, ..
            } => (
                "InitialState".into(),
                format!(
                    "sector={} ships={} missions={}",
                    sector.name,
                    ships.len(),
                    missions.len()
                ),
            ),
            ServerMessage::StateUpdate {
                tick,
                ship_updates,
                ship_spawns,
                events,
                ..
            } => (
                "StateUpdate".into(),
                format!(
                    "tick={} updates={} spawns={} events={}",
                    tick,
                    ship_updates.len(),
                    ship_spawns.len(),
                    events.len()
                ),
            ),
            ServerMessage::ResourceUpdate { reputation, fame, .. } => (
                "ResourceUpdate".into(),
                format!("rep={} fame={}", reputation, fame),
            ),
            ServerMessage::MissionChoice { mission_id, choices, .. } => (
                "MissionChoice".into(),
                format!("mission={} choices={}", mission_id, choices.len()),
            ),
            ServerMessage::MissionResult { mission_id, success, .. } => (
                "MissionResult".into(),
                format!("mission={} success={}", mission_id, success),
            ),
            ServerMessage::CombatUpdate { round, events, is_resolved, .. } => (
                "CombatUpdate".into(),
                format!("round={} events={} resolved={}", round, events.len(), is_resolved),
            ),
            ServerMessage::ChatMessage { sender_name, channel, .. } => (
                "ChatMessage".into(),
                format!("from={} channel={:?}", sender_name, channel),
            ),
            ServerMessage::Error { code, message } => {
                ("Error".into(), format!("[{}] {}", code, message))
            }
            ServerMessage::Pong { server_tick, .. } => {
                ("Pong".into(), format!("tick={}", server_tick))
            }
            ServerMessage::Kicked { reason } => ("Kicked".into(), reason.clone()),
            ServerMessage::SquadronUpdate { message, .. } => {
                ("SquadronUpdate".into(), message.clone())
            }
            ServerMessage::SquadronInvite { squadron_name, .. } => {
                ("SquadronInvite".into(), format!("from={}", squadron_name))
            }
            ServerMessage::AllianceProposal { from_squadron_name, .. } => {
                ("AllianceProposal".into(), format!("from={}", from_squadron_name))
            }
            ServerMessage::HailReceived { from_name, .. } => {
                ("HailReceived".into(), format!("from={}", from_name))
            }
            ServerMessage::TickMetrics(m) => (
                "TickMetrics".into(),
                format!("tick={} {}us", m.tick, m.total_us),
            ),
            ServerMessage::TickMetricsHistory(h) => (
                "TickMetricsHistory".into(),
                format!("ticks={} avg={}us", h.ticks.len(), h.avg_duration_us),
            ),
            ServerMessage::Admin(_) => ("Admin".into(), "admin message".into()),
            ServerMessage::Notification { notification_type, .. } => {
                ("Notification".into(), notification_type.clone())
            }
            ServerMessage::ChoiceRequired { choice_id, .. } => {
                ("ChoiceRequired".into(), choice_id.clone())
            }
        }
    }

    /// Toggle debug panel visibility
    pub fn toggle(&mut self) {
        self.show_debug = !self.show_debug;
    }
}
