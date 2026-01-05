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

/// Statistics about the log buffer state.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct LogBufferStats {
    /// Current number of entries in the buffer
    pub len: usize,
    /// Maximum capacity of the buffer
    pub capacity: usize,
    /// Number of entries that have been dropped due to overflow
    pub dropped: u64,
    /// Number of error-level entries currently in buffer
    pub errors: usize,
    /// Number of warning-level entries currently in buffer
    pub warnings: usize,
}

/// Ring buffer for script logs.
///
/// Keeps the most recent `max_entries` log entries, automatically
/// dropping older entries when the buffer is full.
pub struct ScriptLogBuffer {
    entries: VecDeque<ScriptLogEntry>,
    max_entries: usize,
    /// Count of entries dropped due to buffer overflow
    dropped_count: u64,
}

impl ScriptLogBuffer {
    /// Create a new log buffer with the specified capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
            dropped_count: 0,
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
            self.dropped_count += 1;
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

    /// Get count of entries dropped due to overflow.
    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }

    /// Get the maximum capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.max_entries
    }

    /// Get all statistics in a single call (avoids multiple lock acquisitions).
    pub fn stats(&self) -> LogBufferStats {
        LogBufferStats {
            len: self.entries.len(),
            capacity: self.max_entries,
            dropped: self.dropped_count,
            errors: self.entries.iter().filter(|e| e.level == LogLevel::Error).count(),
            warnings: self.entries.iter().filter(|e| e.level == LogLevel::Warning).count(),
        }
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
        assert_eq!(buffer.dropped_count(), 0);

        // Adding 4th should evict first
        buffer.log(LogLevel::Info, "msg4".into());
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.dropped_count(), 1);

        // Adding 5th should evict second
        buffer.log(LogLevel::Info, "msg5".into());
        assert_eq!(buffer.dropped_count(), 2);

        let entries: Vec<_> = buffer.entries().map(|e| e.message.as_str()).collect();
        assert_eq!(entries, vec!["msg3", "msg4", "msg5"]);
    }

    #[test]
    fn test_stats() {
        let mut buffer = ScriptLogBuffer::new(5);

        buffer.log(LogLevel::Info, "info".into());
        buffer.log(LogLevel::Warning, "warn1".into());
        buffer.log(LogLevel::Warning, "warn2".into());
        buffer.log(LogLevel::Error, "err".into());

        let stats = buffer.stats();
        assert_eq!(stats.len, 4);
        assert_eq!(stats.capacity, 5);
        assert_eq!(stats.dropped, 0);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.warnings, 2);
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
