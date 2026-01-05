//! Script execution log buffer
//!
//! Provides a ring buffer for script execution logs that can be viewed
//! via the admin API for debugging purposes.

use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// A single log entry from script execution.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptLogEntry {
    /// When the log entry was created
    pub timestamp: DateTime<Utc>,
    /// Severity level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Script file that generated this log (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    /// Behavior ID if from a behavior script
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior_id: Option<Uuid>,
}

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Informational message
    Info,
    /// Warning that something might be wrong
    Warning,
    /// Error that prevented operation
    Error,
}

/// Ring buffer for script logs.
///
/// Keeps the most recent `max_entries` log entries, automatically
/// dropping older entries when the buffer is full.
pub struct ScriptLogBuffer {
    entries: VecDeque<ScriptLogEntry>,
    max_entries: usize,
}

impl ScriptLogBuffer {
    /// Create a new log buffer with the specified capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    /// Add a log entry.
    pub fn log(&mut self, level: LogLevel, message: String) {
        let entry = ScriptLogEntry {
            timestamp: Utc::now(),
            level,
            message,
            script_path: None,
            behavior_id: None,
        };

        self.push(entry);
    }

    /// Add a log entry with context.
    pub fn log_with_context(
        &mut self,
        level: LogLevel,
        message: String,
        script_path: Option<String>,
        behavior_id: Option<Uuid>,
    ) {
        let entry = ScriptLogEntry {
            timestamp: Utc::now(),
            level,
            message,
            script_path,
            behavior_id,
        };

        self.push(entry);
    }

    /// Push an entry, evicting oldest if at capacity.
    fn push(&mut self, entry: ScriptLogEntry) {
        if self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get all entries (oldest to newest).
    pub fn entries(&self) -> impl Iterator<Item = &ScriptLogEntry> {
        self.entries.iter()
    }

    /// Get the most recent `count` entries (newest first).
    pub fn recent(&self, count: usize) -> Vec<&ScriptLogEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Get all entries as a Vec (for serialization).
    pub fn all(&self) -> Vec<&ScriptLogEntry> {
        self.entries.iter().collect()
    }

    /// Get total number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get entries filtered by level.
    pub fn filter_by_level(&self, level: LogLevel) -> Vec<&ScriptLogEntry> {
        self.entries.iter().filter(|e| e.level == level).collect()
    }

    /// Get error count.
    pub fn error_count(&self) -> usize {
        self.entries.iter().filter(|e| e.level == LogLevel::Error).count()
    }

    /// Get warning count.
    pub fn warning_count(&self) -> usize {
        self.entries.iter().filter(|e| e.level == LogLevel::Warning).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_capacity() {
        let mut buffer = ScriptLogBuffer::new(3);

        buffer.log(LogLevel::Info, "msg1".into());
        buffer.log(LogLevel::Info, "msg2".into());
        buffer.log(LogLevel::Info, "msg3".into());
        assert_eq!(buffer.len(), 3);

        // Adding 4th should evict first
        buffer.log(LogLevel::Info, "msg4".into());
        assert_eq!(buffer.len(), 3);

        let entries: Vec<_> = buffer.entries().map(|e| e.message.as_str()).collect();
        assert_eq!(entries, vec!["msg2", "msg3", "msg4"]);
    }

    #[test]
    fn test_recent_entries() {
        let mut buffer = ScriptLogBuffer::new(5);

        buffer.log(LogLevel::Info, "a".into());
        buffer.log(LogLevel::Warning, "b".into());
        buffer.log(LogLevel::Error, "c".into());

        let recent: Vec<_> = buffer.recent(2).iter().map(|e| e.message.as_str()).collect();
        assert_eq!(recent, vec!["c", "b"]);
    }
}
