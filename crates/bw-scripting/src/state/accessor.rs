//! State accessor for scripts
//!
//! Provides controlled access to game state with permission checking.
//! The accessor is parameterized by a StateProvider trait to decouple
//! from the concrete GameState implementation.

use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

use bw_core::models::Position;

use super::{
    ShipSnapshot, PlayerSnapshot, SectorSnapshot,
    StateMutation, ShipChanges, PlayerChanges, ShipSpawnConfig,
    MutationResult, EntityType,
};

/// Error type for state access operations.
#[derive(Debug, Clone)]
pub enum AccessError {
    /// Operation not permitted
    PermissionDenied(String),
    /// Entity not found
    NotFound(String),
    /// Invalid operation
    InvalidOperation(String),
}

impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            Self::NotFound(msg) => write!(f, "Not found: {}", msg),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
        }
    }
}

impl std::error::Error for AccessError {}

/// Trait for providing state access. Implemented by the server's GameState.
pub trait StateProvider: Send + Sync {
    /// Get a ship by ID.
    fn get_ship(&self, ship_id: Uuid) -> Option<ShipSnapshot>;

    /// Get all ships in a sector.
    fn get_ships_in_sector(&self, sector_id: Uuid) -> Vec<ShipSnapshot>;

    /// Get ships within range of a position.
    fn get_ships_in_range(&self, sector_id: Uuid, position: Position, range: f64) -> Vec<ShipSnapshot>;

    /// Get a player by ID.
    fn get_player(&self, player_id: Uuid) -> Option<PlayerSnapshot>;

    /// Get a sector by ID.
    fn get_sector(&self, sector_id: Uuid) -> Option<SectorSnapshot>;

    /// Apply mutations to game state. Returns results for each mutation.
    fn apply_mutations(&self, mutations: Vec<StateMutation>) -> Vec<MutationResult>;
}

/// Access permissions for scripts.
#[derive(Debug, Clone)]
pub struct AccessPermissions {
    /// Can read ship data
    pub can_read_ships: bool,
    /// Can modify ship data
    pub can_write_ships: bool,
    /// Can read player data
    pub can_read_players: bool,
    /// Can modify player data
    pub can_write_players: bool,
    /// Can spawn new entities
    pub can_spawn_entities: bool,
    /// Can destroy entities
    pub can_destroy_entities: bool,
    /// Can emit events
    pub can_emit_events: bool,
    /// Restrict access to specific sectors (None = all sectors)
    pub allowed_sectors: Option<Vec<Uuid>>,
    /// The entity this script is attached to (for behavior scripts)
    pub owner_entity_id: Option<Uuid>,
}

impl Default for AccessPermissions {
    fn default() -> Self {
        Self {
            can_read_ships: true,
            can_write_ships: false,
            can_read_players: true,
            can_write_players: false,
            can_spawn_entities: false,
            can_destroy_entities: false,
            can_emit_events: false,
            allowed_sectors: None,
            owner_entity_id: None,
        }
    }
}

impl AccessPermissions {
    /// Full permissions for trusted scripts (e.g., built-in AI behaviors).
    pub fn trusted() -> Self {
        Self {
            can_read_ships: true,
            can_write_ships: true,
            can_read_players: true,
            can_write_players: false, // Still restricted
            can_spawn_entities: true,
            can_destroy_entities: true,
            can_emit_events: true,
            allowed_sectors: None,
            owner_entity_id: None,
        }
    }

    /// Permissions for NPC behavior scripts.
    pub fn npc_behavior(entity_id: Uuid, sector_id: Uuid) -> Self {
        Self {
            can_read_ships: true,
            can_write_ships: true, // Can modify own ship
            can_read_players: true,
            can_write_players: false,
            can_spawn_entities: false,
            can_destroy_entities: false,
            can_emit_events: true,
            allowed_sectors: Some(vec![sector_id]),
            owner_entity_id: Some(entity_id),
        }
    }

    /// Read-only permissions for queries.
    pub fn read_only() -> Self {
        Self {
            can_read_ships: true,
            can_write_ships: false,
            can_read_players: true,
            can_write_players: false,
            can_spawn_entities: false,
            can_destroy_entities: false,
            can_emit_events: false,
            allowed_sectors: None,
            owner_entity_id: None,
        }
    }

    /// Check if sector access is allowed.
    pub fn can_access_sector(&self, sector_id: Uuid) -> bool {
        match &self.allowed_sectors {
            Some(sectors) => sectors.contains(&sector_id),
            None => true,
        }
    }
}

/// State accessor that scripts use to interact with game state.
///
/// Provides read access via snapshots and queues mutations for batch application.
pub struct StateAccessor {
    provider: Arc<dyn StateProvider>,
    permissions: RwLock<AccessPermissions>,
    pending_mutations: RwLock<Vec<StateMutation>>,
    /// Current context sector (for relative queries)
    context_sector_id: RwLock<Option<Uuid>>,
}

impl StateAccessor {
    /// Create a new state accessor with the given provider.
    pub fn new(provider: Arc<dyn StateProvider>) -> Self {
        Self {
            provider,
            permissions: RwLock::new(AccessPermissions::default()),
            pending_mutations: RwLock::new(Vec::new()),
            context_sector_id: RwLock::new(None),
        }
    }

    /// Set the current permissions.
    pub fn set_permissions(&self, permissions: AccessPermissions) {
        *self.permissions.write() = permissions;
    }

    /// Get current permissions.
    pub fn permissions(&self) -> AccessPermissions {
        self.permissions.read().clone()
    }

    /// Set the context sector for relative queries.
    pub fn set_context_sector(&self, sector_id: Uuid) {
        *self.context_sector_id.write() = Some(sector_id);
    }

    /// Get a ship by ID.
    pub fn get_ship(&self, ship_id: Uuid) -> Result<Option<ShipSnapshot>, AccessError> {
        let perms = self.permissions.read();
        if !perms.can_read_ships {
            return Err(AccessError::PermissionDenied("Cannot read ships".into()));
        }

        let snapshot = self.provider.get_ship(ship_id);

        // Check sector access
        if let Some(ref ship) = snapshot {
            if !perms.can_access_sector(ship.sector_id) {
                return Err(AccessError::PermissionDenied(
                    "Cannot access ships in this sector".into()
                ));
            }
        }

        Ok(snapshot)
    }

    /// Get all ships in a sector.
    pub fn get_ships_in_sector(&self, sector_id: Uuid) -> Result<Vec<ShipSnapshot>, AccessError> {
        let perms = self.permissions.read();
        if !perms.can_read_ships {
            return Err(AccessError::PermissionDenied("Cannot read ships".into()));
        }
        if !perms.can_access_sector(sector_id) {
            return Err(AccessError::PermissionDenied(
                "Cannot access this sector".into()
            ));
        }

        Ok(self.provider.get_ships_in_sector(sector_id))
    }

    /// Get ships within range of a position.
    pub fn get_ships_in_range(
        &self,
        sector_id: Uuid,
        position: Position,
        range: f64,
    ) -> Result<Vec<ShipSnapshot>, AccessError> {
        let perms = self.permissions.read();
        if !perms.can_read_ships {
            return Err(AccessError::PermissionDenied("Cannot read ships".into()));
        }
        if !perms.can_access_sector(sector_id) {
            return Err(AccessError::PermissionDenied(
                "Cannot access this sector".into()
            ));
        }

        Ok(self.provider.get_ships_in_range(sector_id, position, range))
    }

    /// Get a player by ID.
    pub fn get_player(&self, player_id: Uuid) -> Result<Option<PlayerSnapshot>, AccessError> {
        let perms = self.permissions.read();
        if !perms.can_read_players {
            return Err(AccessError::PermissionDenied("Cannot read players".into()));
        }

        Ok(self.provider.get_player(player_id))
    }

    /// Get a sector by ID.
    pub fn get_sector(&self, sector_id: Uuid) -> Result<Option<SectorSnapshot>, AccessError> {
        let perms = self.permissions.read();
        if !perms.can_access_sector(sector_id) {
            return Err(AccessError::PermissionDenied(
                "Cannot access this sector".into()
            ));
        }

        Ok(self.provider.get_sector(sector_id))
    }

    /// Get the current context sector.
    pub fn get_context_sector(&self) -> Result<Option<SectorSnapshot>, AccessError> {
        let sector_id = *self.context_sector_id.read();
        match sector_id {
            Some(id) => self.get_sector(id),
            None => Ok(None),
        }
    }

    /// Queue a ship modification.
    pub fn modify_ship(&self, ship_id: Uuid, changes: ShipChanges) -> Result<(), AccessError> {
        let perms = self.permissions.read();
        if !perms.can_write_ships {
            // Check if modifying own entity
            if perms.owner_entity_id != Some(ship_id) {
                return Err(AccessError::PermissionDenied(
                    "Cannot modify ships".into()
                ));
            }
        }

        if changes.is_empty() {
            return Ok(());
        }

        self.pending_mutations.write().push(StateMutation::ModifyShip {
            ship_id,
            changes,
        });

        Ok(())
    }

    /// Queue a player modification.
    pub fn modify_player(&self, player_id: Uuid, changes: PlayerChanges) -> Result<(), AccessError> {
        let perms = self.permissions.read();
        if !perms.can_write_players {
            return Err(AccessError::PermissionDenied("Cannot modify players".into()));
        }

        if changes.is_empty() {
            return Ok(());
        }

        self.pending_mutations.write().push(StateMutation::ModifyPlayer {
            player_id,
            changes,
        });

        Ok(())
    }

    /// Queue an entity spawn.
    pub fn spawn_ship(&self, config: ShipSpawnConfig) -> Result<(), AccessError> {
        let perms = self.permissions.read();
        if !perms.can_spawn_entities {
            return Err(AccessError::PermissionDenied("Cannot spawn entities".into()));
        }
        if !perms.can_access_sector(config.sector_id) {
            return Err(AccessError::PermissionDenied(
                "Cannot spawn in this sector".into()
            ));
        }

        self.pending_mutations.write().push(StateMutation::SpawnShip { config });

        Ok(())
    }

    /// Queue an entity destruction.
    pub fn destroy_entity(&self, entity_id: Uuid, entity_type: EntityType) -> Result<(), AccessError> {
        let perms = self.permissions.read();
        if !perms.can_destroy_entities {
            return Err(AccessError::PermissionDenied("Cannot destroy entities".into()));
        }

        self.pending_mutations.write().push(StateMutation::DestroyEntity {
            entity_id,
            entity_type,
        });

        Ok(())
    }

    /// Queue an event emission.
    pub fn emit_event(
        &self,
        event_type: String,
        data: rhai::Map,
        actor_id: Option<Uuid>,
        target_id: Option<Uuid>,
    ) -> Result<(), AccessError> {
        let perms = self.permissions.read();
        if !perms.can_emit_events {
            return Err(AccessError::PermissionDenied("Cannot emit events".into()));
        }

        self.pending_mutations.write().push(StateMutation::EmitEvent {
            event_type,
            data,
            actor_id,
            target_id,
        });

        Ok(())
    }

    /// Take all pending mutations (clears the queue).
    pub fn take_mutations(&self) -> Vec<StateMutation> {
        std::mem::take(&mut *self.pending_mutations.write())
    }

    /// Apply all pending mutations and clear the queue.
    pub fn apply_pending_mutations(&self) -> Vec<MutationResult> {
        let mutations = self.take_mutations();
        if mutations.is_empty() {
            return Vec::new();
        }
        self.provider.apply_mutations(mutations)
    }

    /// Get count of pending mutations.
    pub fn pending_mutation_count(&self) -> usize {
        self.pending_mutations.read().len()
    }

    /// Clear pending mutations without applying.
    pub fn clear_mutations(&self) {
        self.pending_mutations.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock provider for testing
    struct MockProvider;

    impl StateProvider for MockProvider {
        fn get_ship(&self, _ship_id: Uuid) -> Option<ShipSnapshot> {
            None
        }

        fn get_ships_in_sector(&self, _sector_id: Uuid) -> Vec<ShipSnapshot> {
            Vec::new()
        }

        fn get_ships_in_range(&self, _sector_id: Uuid, _position: Position, _range: f64) -> Vec<ShipSnapshot> {
            Vec::new()
        }

        fn get_player(&self, _player_id: Uuid) -> Option<PlayerSnapshot> {
            None
        }

        fn get_sector(&self, _sector_id: Uuid) -> Option<SectorSnapshot> {
            None
        }

        fn apply_mutations(&self, mutations: Vec<StateMutation>) -> Vec<MutationResult> {
            mutations.into_iter()
                .map(|m| MutationResult::success(m))
                .collect()
        }
    }

    #[test]
    fn test_permissions_default() {
        let perms = AccessPermissions::default();
        assert!(perms.can_read_ships);
        assert!(!perms.can_write_ships);
    }

    #[test]
    fn test_accessor_permission_denied() {
        let accessor = StateAccessor::new(Arc::new(MockProvider));
        accessor.set_permissions(AccessPermissions::read_only());

        let result = accessor.modify_ship(Uuid::new_v4(), ShipChanges::default());
        assert!(matches!(result, Err(AccessError::PermissionDenied(_))));
    }
}
