//! Performance metrics collection for tick composition analysis

use std::collections::VecDeque;
use std::time::Instant;

use parking_lot::RwLock;
use uuid::Uuid;

use bw_shared::dto::{PhaseTimingDto, SectorTimingDto, TickMetricsDto, TickMetricsHistoryDto};

/// Maximum number of ticks to keep in history
const METRICS_HISTORY_SIZE: usize = 100;

/// Builder for collecting timing data during a single tick
pub struct TickMetricsBuilder {
    tick: u64,
    tick_start: Instant,
    phases: Vec<PhaseTimingDto>,
    sectors: Vec<SectorTimingDto>,
    current_phase_start: Option<Instant>,
    current_phase_name: Option<String>,
}

impl TickMetricsBuilder {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            tick_start: Instant::now(),
            phases: Vec::new(),
            sectors: Vec::new(),
            current_phase_start: None,
            current_phase_name: None,
        }
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, name: &str) {
        self.end_phase(); // End any previous phase
        self.current_phase_start = Some(Instant::now());
        self.current_phase_name = Some(name.to_string());
    }

    /// End the current phase
    pub fn end_phase(&mut self) {
        if let (Some(start), Some(name)) = (self.current_phase_start.take(), self.current_phase_name.take()) {
            self.phases.push(PhaseTimingDto {
                name,
                duration_us: start.elapsed().as_micros() as u64,
                skipped: false,
            });
        }
    }

    /// Record a skipped phase (for conditional phases that didn't run this tick)
    #[allow(dead_code)]
    pub fn record_skipped(&mut self, name: &str) {
        self.phases.push(PhaseTimingDto {
            name: name.to_string(),
            duration_us: 0,
            skipped: true,
        });
    }

    /// Record a complete sector timing
    pub fn add_sector_timing(&mut self, timing: SectorTimingDto) {
        self.sectors.push(timing);
    }

    /// Finalize and build the metrics DTO
    pub fn finish(mut self) -> TickMetricsDto {
        self.end_phase();
        let total_us = self.tick_start.elapsed().as_micros() as u64;
        let budget_us = 100_000; // 100ms

        TickMetricsDto {
            tick: self.tick,
            total_us,
            budget_us,
            over_budget: total_us > budget_us,
            phases: self.phases,
            sectors: self.sectors,
        }
    }
}

/// Builder for collecting per-sector timing
pub struct SectorMetricsBuilder {
    sector_id: Uuid,
    sector_name: String,
    sector_start: Instant,
    phases: Vec<PhaseTimingDto>,
    current_phase_start: Option<Instant>,
    current_phase_name: Option<String>,
}

impl SectorMetricsBuilder {
    pub fn new(sector_id: Uuid, sector_name: String) -> Self {
        Self {
            sector_id,
            sector_name,
            sector_start: Instant::now(),
            phases: Vec::new(),
            current_phase_start: None,
            current_phase_name: None,
        }
    }

    pub fn start_phase(&mut self, name: &str) {
        self.end_phase();
        self.current_phase_start = Some(Instant::now());
        self.current_phase_name = Some(name.to_string());
    }

    pub fn end_phase(&mut self) {
        if let (Some(start), Some(name)) = (self.current_phase_start.take(), self.current_phase_name.take()) {
            self.phases.push(PhaseTimingDto {
                name,
                duration_us: start.elapsed().as_micros() as u64,
                skipped: false,
            });
        }
    }

    /// Record a skipped phase (for conditional phases that didn't run this tick)
    pub fn record_skipped(&mut self, name: &str) {
        self.phases.push(PhaseTimingDto {
            name: name.to_string(),
            duration_us: 0,
            skipped: true,
        });
    }

    pub fn finish(mut self) -> SectorTimingDto {
        self.end_phase();
        SectorTimingDto {
            sector_id: self.sector_id,
            sector_name: self.sector_name,
            total_us: self.sector_start.elapsed().as_micros() as u64,
            phases: self.phases,
        }
    }
}

/// Thread-safe metrics storage with rolling window
pub struct MetricsStore {
    history: RwLock<VecDeque<TickMetricsDto>>,
    /// Players subscribed to metrics updates
    subscribers: RwLock<Vec<Uuid>>,
}

impl MetricsStore {
    pub fn new() -> Self {
        Self {
            history: RwLock::new(VecDeque::with_capacity(METRICS_HISTORY_SIZE)),
            subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Record a completed tick's metrics
    pub fn record(&self, metrics: TickMetricsDto) {
        let mut history = self.history.write();
        if history.len() >= METRICS_HISTORY_SIZE {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    /// Get the latest tick metrics
    #[allow(dead_code)]
    pub fn latest(&self) -> Option<TickMetricsDto> {
        self.history.read().back().cloned()
    }

    /// Get complete history with statistics
    pub fn get_history(&self) -> TickMetricsHistoryDto {
        let history = self.history.read();
        let ticks: Vec<TickMetricsDto> = history.iter().cloned().collect();

        let durations: Vec<u64> = ticks.iter().map(|t| t.total_us).collect();
        let avg = if durations.is_empty() {
            0
        } else {
            durations.iter().sum::<u64>() / durations.len() as u64
        };

        let mut sorted = durations.clone();
        sorted.sort();
        let p95 = sorted
            .get((sorted.len() as f64 * 0.95) as usize)
            .copied()
            .unwrap_or(0);
        let max = sorted.last().copied().unwrap_or(0);
        let over_budget = ticks.iter().filter(|t| t.over_budget).count() as u32;

        TickMetricsHistoryDto {
            ticks,
            avg_duration_us: avg,
            p95_duration_us: p95,
            max_duration_us: max,
            over_budget_count: over_budget,
        }
    }

    /// Subscribe a player to metrics updates
    pub fn subscribe(&self, player_id: Uuid) {
        let mut subs = self.subscribers.write();
        if !subs.contains(&player_id) {
            subs.push(player_id);
        }
    }

    /// Unsubscribe a player from metrics updates
    pub fn unsubscribe(&self, player_id: Uuid) {
        let mut subs = self.subscribers.write();
        subs.retain(|&id| id != player_id);
    }

    /// Get list of subscribers
    pub fn get_subscribers(&self) -> Vec<Uuid> {
        self.subscribers.read().clone()
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self::new()
    }
}
