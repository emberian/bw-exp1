//! SectorView facade for scripts
//!
//! Provides fluent property access for sectors:
//! ```rhai
//! let sector = sector(sector_id);
//! if sector.danger_level == "hostile" {
//!     sector.broadcast("Warning: Hostile sector!");
//! }
//! for loc in sector.locations {
//!     // ...
//! }
//! ```

use rhai::{Dynamic, Engine, Array, CustomType, TypeBuilder};
use uuid::Uuid;

use crate::context::with_accessor;
use super::LocationView;

/// A facade view of a sector for scripts.
///
/// Provides natural property access for sector data.
#[derive(Debug, Clone)]
pub struct SectorView {
    pub(crate) id: Uuid,
}

impl SectorView {
    /// Create a new sector view.
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

    /// Get sector ID as string.
    pub fn get_id(&mut self) -> String {
        self.id.to_string()
    }

    /// Get sector name.
    pub fn get_name(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| s.name.clone())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get danger level (safe, moderate, dangerous, hostile).
    pub fn get_danger_level(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| s.danger_level.clone())
                .unwrap_or_else(|| "safe".to_string())
        }).unwrap_or_else(|| "safe".to_string())
    }

    /// Get traffic density.
    pub fn get_traffic_density(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| s.traffic_density.clone())
                .unwrap_or_else(|| "moderate".to_string())
        }).unwrap_or_else(|| "moderate".to_string())
    }

    /// Check if this is a core sector.
    pub fn get_is_core_sector(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| s.is_core_sector)
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Get controlling faction ID.
    pub fn get_controlling_faction(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .and_then(|s| s.controlling_faction)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get controlling squadron ID.
    pub fn get_controlling_squadron(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .and_then(|s| s.controlling_squadron)
                .map(|id| id.to_string())
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get locations as array of LocationViews.
    pub fn get_locations(&mut self) -> Array {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    s.locations.iter()
                        .map(|loc| Dynamic::from(LocationView::new(loc.id, self.id)))
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get location count.
    pub fn get_location_count(&mut self) -> i64 {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| s.locations.len() as i64)
                .unwrap_or(0)
        }).unwrap_or(0)
    }

    // =========================================================================
    // Methods
    // =========================================================================

    /// Broadcast a message to all players in this sector.
    pub fn broadcast(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.broadcast_to_sector(self.id, message, "info".to_string());
        });
    }

    /// Broadcast a warning to all players in this sector.
    pub fn broadcast_warning(&mut self, message: String) {
        with_accessor(|accessor| {
            let _ = accessor.broadcast_to_sector(self.id, message, "warning".to_string());
        });
    }

    /// Check if sector exists.
    pub fn exists(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_sector(self.id).ok().flatten().is_some()
        }).unwrap_or(false)
    }

    /// Check if sector is dangerous (dangerous or hostile).
    pub fn is_dangerous(&mut self) -> bool {
        let level = self.get_danger_level();
        level == "dangerous" || level == "hostile"
    }

    /// Check if sector is safe.
    pub fn is_safe(&mut self) -> bool {
        self.get_danger_level() == "safe"
    }

    /// Find a location by type.
    pub fn find_location_by_type(&mut self, location_type: String) -> Dynamic {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.location_type == location_type)
                        .map(|loc| Dynamic::from(LocationView::new(loc.id, self.id)))
                })
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    }

    /// Find all locations of a given type.
    pub fn find_locations_by_type(&mut self, location_type: String) -> Array {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    s.locations.iter()
                        .filter(|loc| loc.location_type == location_type)
                        .map(|loc| Dynamic::from(LocationView::new(loc.id, self.id)))
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Find a dockable location (station, outpost, etc).
    pub fn find_dockable(&mut self) -> Dynamic {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.is_dockable)
                        .map(|loc| Dynamic::from(LocationView::new(loc.id, self.id)))
                })
                .unwrap_or(Dynamic::UNIT)
        }).unwrap_or(Dynamic::UNIT)
    }

    /// Find all dockable locations.
    pub fn find_all_dockable(&mut self) -> Array {
        with_accessor(|accessor| {
            accessor.get_sector(self.id)
                .ok()
                .flatten()
                .map(|s| {
                    s.locations.iter()
                        .filter(|loc| loc.is_dockable)
                        .map(|loc| Dynamic::from(LocationView::new(loc.id, self.id)))
                        .collect()
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }
}

// Implement CustomType for automatic registration
impl CustomType for SectorView {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("SectorView")
            .with_get("id", Self::get_id)
            .with_get("name", Self::get_name)
            .with_get("danger_level", Self::get_danger_level)
            .with_get("traffic_density", Self::get_traffic_density)
            .with_get("is_core_sector", Self::get_is_core_sector)
            .with_get("controlling_faction", Self::get_controlling_faction)
            .with_get("controlling_squadron", Self::get_controlling_squadron)
            .with_get("locations", Self::get_locations)
            .with_get("location_count", Self::get_location_count);
    }
}

/// Register SectorView with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register the type with automatic getters/setters
    engine.build_type::<SectorView>();

    // Register methods
    engine.register_fn("broadcast", SectorView::broadcast);
    engine.register_fn("broadcast_warning", SectorView::broadcast_warning);
    engine.register_fn("exists", SectorView::exists);
    engine.register_fn("is_dangerous", SectorView::is_dangerous);
    engine.register_fn("is_safe", SectorView::is_safe);
    engine.register_fn("find_location_by_type", SectorView::find_location_by_type);
    engine.register_fn("find_locations_by_type", SectorView::find_locations_by_type);
    engine.register_fn("find_dockable", SectorView::find_dockable);
    engine.register_fn("find_all_dockable", SectorView::find_all_dockable);

    // Constructor from string ID
    engine.register_fn("sector", |id: String| -> Dynamic {
        match SectorView::from_string(&id) {
            Some(view) => Dynamic::from(view),
            None => Dynamic::UNIT,
        }
    });
}
