//! LocationView facade for scripts
//!
//! Provides fluent property access for locations (stations, gates, etc):
//! ```rhai
//! let station = sector.find_dockable();
//! if station.is_dockable {
//!     let prices = station.get_market_prices();
//! }
//! ```

use rhai::{Dynamic, Engine, Map, CustomType, TypeBuilder};
use uuid::Uuid;

use crate::context::with_accessor;

/// A facade view of a location (station, gate, etc) for scripts.
///
/// Provides natural property access for location data.
#[derive(Debug, Clone)]
pub struct LocationView {
    pub(crate) id: Uuid,
    pub(crate) sector_id: Uuid,
}

impl LocationView {
    /// Create a new location view.
    pub fn new(id: Uuid, sector_id: Uuid) -> Self {
        Self { id, sector_id }
    }

    /// Create from string UUIDs.
    pub fn from_strings(id: &str, sector_id: &str) -> Option<Self> {
        let id = Uuid::parse_str(id).ok()?;
        let sector_id = Uuid::parse_str(sector_id).ok()?;
        Some(Self::new(id, sector_id))
    }

    // =========================================================================
    // Getters - read from snapshot via accessor
    // =========================================================================

    /// Get location ID as string.
    pub fn get_id(&mut self) -> String {
        self.id.to_string()
    }

    /// Get sector ID as string.
    pub fn get_sector_id(&mut self) -> String {
        self.sector_id.to_string()
    }

    /// Get location name.
    pub fn get_name(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .map(|loc| loc.name.clone())
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get location type.
    pub fn get_location_type(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .map(|loc| loc.location_type.clone())
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get position as a map with x, y, z.
    pub fn get_position(&mut self) -> Map {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .map(|loc| {
                            let mut map = Map::new();
                            map.insert("x".into(), Dynamic::from(loc.position.x));
                            map.insert("y".into(), Dynamic::from(loc.position.y));
                            map.insert("z".into(), Dynamic::from(loc.position.z));
                            map
                        })
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Get faction ID.
    pub fn get_faction_id(&mut self) -> String {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .and_then(|loc| loc.faction_id)
                        .map(|id| id.to_string())
                })
                .unwrap_or_default()
        }).unwrap_or_default()
    }

    /// Check if location is active.
    pub fn get_is_active(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .map(|loc| loc.is_active)
                })
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Check if location is dockable.
    pub fn get_is_dockable(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .and_then(|s| {
                    s.locations.iter()
                        .find(|loc| loc.id == self.id)
                        .map(|loc| loc.is_dockable)
                })
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    // =========================================================================
    // Methods
    // =========================================================================

    /// Check if location exists.
    pub fn exists(&mut self) -> bool {
        with_accessor(|accessor| {
            accessor.get_sector(self.sector_id)
                .ok()
                .flatten()
                .map(|s| s.locations.iter().any(|loc| loc.id == self.id))
                .unwrap_or(false)
        }).unwrap_or(false)
    }

    /// Check if this is a station type.
    pub fn is_station(&mut self) -> bool {
        let loc_type = self.get_location_type();
        matches!(loc_type.as_str(),
            "naval_station" | "civilian_station" | "free_port"
        )
    }

    /// Check if this is a resource location.
    pub fn is_resource(&mut self) -> bool {
        let loc_type = self.get_location_type();
        matches!(loc_type.as_str(),
            "mining_facility" | "processing_plant" | "orbital_factory" | "asteroid_field"
        )
    }

    /// Check if this is a jump gate.
    pub fn is_gate(&mut self) -> bool {
        self.get_location_type() == "jumpgate"
    }

    /// Check if this is an anomaly or special location.
    pub fn is_special(&mut self) -> bool {
        let loc_type = self.get_location_type();
        matches!(loc_type.as_str(),
            "anomaly" | "graveyard" | "debris_field" | "archive"
        )
    }

    /// Get X coordinate.
    pub fn get_x(&mut self) -> f64 {
        let pos = self.get_position();
        pos.get("x")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(0.0)
    }

    /// Get Y coordinate.
    pub fn get_y(&mut self) -> f64 {
        let pos = self.get_position();
        pos.get("y")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(0.0)
    }

    /// Get Z coordinate.
    pub fn get_z(&mut self) -> f64 {
        let pos = self.get_position();
        pos.get("z")
            .and_then(|v| v.as_float().ok())
            .unwrap_or(0.0)
    }

    /// Calculate distance to a position.
    pub fn distance_to(&mut self, x: f64, y: f64, z: f64) -> f64 {
        let lx = self.get_x();
        let ly = self.get_y();
        let lz = self.get_z();
        let dx = lx - x;
        let dy = ly - y;
        let dz = lz - z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

// Implement CustomType for automatic registration
impl CustomType for LocationView {
    fn build(mut builder: TypeBuilder<Self>) {
        builder
            .with_name("LocationView")
            .with_get("id", Self::get_id)
            .with_get("sector_id", Self::get_sector_id)
            .with_get("name", Self::get_name)
            .with_get("location_type", Self::get_location_type)
            .with_get("position", Self::get_position)
            .with_get("faction_id", Self::get_faction_id)
            .with_get("is_active", Self::get_is_active)
            .with_get("is_dockable", Self::get_is_dockable)
            .with_get("x", Self::get_x)
            .with_get("y", Self::get_y)
            .with_get("z", Self::get_z);
    }
}

/// Register LocationView with the Rhai engine.
pub fn register(engine: &mut Engine) {
    // Register the type with automatic getters/setters
    engine.build_type::<LocationView>();

    // Register methods
    engine.register_fn("exists", LocationView::exists);
    engine.register_fn("is_station", LocationView::is_station);
    engine.register_fn("is_resource", LocationView::is_resource);
    engine.register_fn("is_gate", LocationView::is_gate);
    engine.register_fn("is_special", LocationView::is_special);
    engine.register_fn("distance_to", LocationView::distance_to);

    // Constructor from string IDs
    engine.register_fn("location", |id: String, sector_id: String| -> Dynamic {
        match LocationView::from_strings(&id, &sector_id) {
            Some(view) => Dynamic::from(view),
            None => Dynamic::UNIT,
        }
    });
}
