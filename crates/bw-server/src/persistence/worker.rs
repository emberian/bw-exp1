//! Streaming persistence worker
//!
//! Provides near-real-time persistence with batched writes.
//! Changes are sent via channel and persisted every 100ms or when batch fills.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sea_orm::{DatabaseConnection, Set};
use tokio::sync::mpsc;
use uuid::Uuid;

use bw_core::models::{Mission, Player, Ship, Squadron};

use super::entities;

/// Commands sent to the persistence worker.
#[derive(Debug, Clone)]
pub enum PersistCmd {
    /// Upsert a player (insert or update).
    UpsertPlayer(Player, String), // Player + password_hash
    /// Update a player (no password change).
    UpdatePlayer(Player),
    /// Upsert a ship.
    UpsertShip(Ship),
    /// Delete a ship.
    DeleteShip(Uuid),
    /// Upsert a squadron.
    UpsertSquadron(Squadron),
    /// Update a mission.
    UpdateMission(Mission),
    /// Flush all pending writes immediately.
    Flush,
    /// Shutdown the worker.
    Shutdown,
}

/// Handle for sending persistence commands.
#[derive(Clone)]
pub struct Persistence {
    tx: mpsc::Sender<PersistCmd>,
}

impl Persistence {
    /// Create a new persistence handle.
    pub fn new(tx: mpsc::Sender<PersistCmd>) -> Self {
        Self { tx }
    }

    /// Persist a player update (password unchanged).
    pub fn persist_player(&self, player: Player) {
        let _ = self.tx.try_send(PersistCmd::UpdatePlayer(player));
    }

    /// Persist a new player with password.
    pub fn persist_new_player(&self, player: Player, password_hash: String) {
        let _ = self.tx.try_send(PersistCmd::UpsertPlayer(player, password_hash));
    }

    /// Persist a ship update.
    pub fn persist_ship(&self, ship: Ship) {
        let _ = self.tx.try_send(PersistCmd::UpsertShip(ship));
    }

    /// Persist ship deletion.
    pub fn delete_ship(&self, ship_id: Uuid) {
        let _ = self.tx.try_send(PersistCmd::DeleteShip(ship_id));
    }

    /// Persist a squadron update.
    pub fn persist_squadron(&self, squadron: Squadron) {
        let _ = self.tx.try_send(PersistCmd::UpsertSquadron(squadron));
    }

    /// Persist a mission update.
    pub fn persist_mission(&self, mission: Mission) {
        let _ = self.tx.try_send(PersistCmd::UpdateMission(mission));
    }

    /// Request immediate flush of pending writes.
    pub fn flush(&self) {
        let _ = self.tx.try_send(PersistCmd::Flush);
    }

    /// Request worker shutdown.
    pub fn shutdown(&self) {
        let _ = self.tx.try_send(PersistCmd::Shutdown);
    }
}

/// Batch of pending writes, deduplicated by entity ID.
#[derive(Default)]
struct WriteBatch {
    players: HashMap<Uuid, (Player, Option<String>)>,
    ships: HashMap<Uuid, Option<Ship>>, // None = delete
    squadrons: HashMap<Uuid, Squadron>,
    missions: HashMap<Uuid, Mission>,
}

impl WriteBatch {
    fn is_empty(&self) -> bool {
        self.players.is_empty()
            && self.ships.is_empty()
            && self.squadrons.is_empty()
            && self.missions.is_empty()
    }

    fn len(&self) -> usize {
        self.players.len() + self.ships.len() + self.squadrons.len() + self.missions.len()
    }

    fn clear(&mut self) {
        self.players.clear();
        self.ships.clear();
        self.squadrons.clear();
        self.missions.clear();
    }
}

/// Spawn the persistence worker.
///
/// Returns a handle for sending commands.
pub fn spawn_persistence_worker(db: DatabaseConnection) -> Persistence {
    let (tx, rx) = mpsc::channel(1000);
    tokio::spawn(persistence_worker(rx, db));
    Persistence::new(tx)
}

/// The persistence worker loop.
async fn persistence_worker(mut rx: mpsc::Receiver<PersistCmd>, db: DatabaseConnection) {
    let mut batch = WriteBatch::default();
    let flush_interval = Duration::from_millis(100);
    let max_batch_size = 50;

    tracing::info!("Persistence worker started (flush interval: {:?})", flush_interval);

    loop {
        let deadline = Instant::now() + flush_interval;

        // Collect commands until deadline or batch is full
        loop {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                break;
            }

            tokio::select! {
                cmd = rx.recv() => {
                    match cmd {
                        Some(PersistCmd::Shutdown) | None => {
                            // Flush remaining and exit
                            if !batch.is_empty() {
                                flush_batch(&db, &mut batch).await;
                            }
                            tracing::info!("Persistence worker shutting down");
                            return;
                        }
                        Some(PersistCmd::Flush) => {
                            if !batch.is_empty() {
                                flush_batch(&db, &mut batch).await;
                            }
                        }
                        Some(cmd) => {
                            apply_command(&mut batch, cmd);
                            if batch.len() >= max_batch_size {
                                flush_batch(&db, &mut batch).await;
                            }
                        }
                    }
                }
                _ = tokio::time::sleep(timeout) => {
                    break;
                }
            }
        }

        // Flush on deadline
        if !batch.is_empty() {
            flush_batch(&db, &mut batch).await;
        }
    }
}

/// Apply a command to the batch.
fn apply_command(batch: &mut WriteBatch, cmd: PersistCmd) {
    match cmd {
        PersistCmd::UpsertPlayer(player, password_hash) => {
            batch.players.insert(player.id, (player, Some(password_hash)));
        }
        PersistCmd::UpdatePlayer(player) => {
            // Preserve existing password hash if any
            let password_hash = batch.players.get(&player.id).and_then(|(_, h)| h.clone());
            batch.players.insert(player.id, (player, password_hash));
        }
        PersistCmd::UpsertShip(ship) => {
            batch.ships.insert(ship.id, Some(ship));
        }
        PersistCmd::DeleteShip(id) => {
            batch.ships.insert(id, None);
        }
        PersistCmd::UpsertSquadron(squadron) => {
            batch.squadrons.insert(squadron.id, squadron);
        }
        PersistCmd::UpdateMission(mission) => {
            batch.missions.insert(mission.id, mission);
        }
        PersistCmd::Flush | PersistCmd::Shutdown => {}
    }
}

/// Flush the batch to the database.
async fn flush_batch(db: &DatabaseConnection, batch: &mut WriteBatch) {
    let start = Instant::now();
    let mut success_count = 0;
    let mut error_count = 0;

    // Flush players
    for (id, (player, password_hash)) in batch.players.drain() {
        match upsert_player(db, &player, password_hash.as_deref()).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::error!("Failed to persist player {}: {}", id, e);
            }
        }
    }

    // Flush ships
    for (id, ship_opt) in batch.ships.drain() {
        let result = match ship_opt {
            Some(ship) => upsert_ship(db, &ship).await,
            None => delete_ship(db, id).await,
        };
        match result {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::error!("Failed to persist ship {}: {}", id, e);
            }
        }
    }

    // Flush squadrons
    for (id, squadron) in batch.squadrons.drain() {
        match upsert_squadron(db, &squadron).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::error!("Failed to persist squadron {}: {}", id, e);
            }
        }
    }

    // Flush missions
    for (id, mission) in batch.missions.drain() {
        match upsert_mission(db, &mission).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                error_count += 1;
                tracing::error!("Failed to persist mission {}: {}", id, e);
            }
        }
    }

    let elapsed = start.elapsed();
    if error_count > 0 {
        tracing::warn!(
            "Persistence flush: {} succeeded, {} failed in {:?}",
            success_count, error_count, elapsed
        );
    } else if success_count > 0 {
        tracing::debug!("Persistence flush: {} writes in {:?}", success_count, elapsed);
    }

    batch.clear();
}

// ============================================================================
// Domain model to SeaORM active model conversions
// ============================================================================

async fn upsert_player(
    db: &DatabaseConnection,
    player: &Player,
    password_hash: Option<&str>,
) -> Result<(), sea_orm::DbErr> {
    use entities::player;
    use sea_orm::EntityTrait;

    let model = player::ActiveModel {
        id: Set(player.id.to_string()),
        username: Set(player.username.clone()),
        password_hash: if let Some(hash) = password_hash {
            Set(hash.to_string())
        } else {
            sea_orm::ActiveValue::NotSet
        },
        reputation: Set(player.resources.reputation),
        fame: Set(player.resources.fame),
        faction_standings: Set(serde_json::to_string(&player.faction_standings).unwrap_or_default()),
        stats: Set(serde_json::to_string(&player.stats).unwrap_or_default()),
        squadron_id: Set(player.squadron_id.map(|id| id.to_string())),
        squadron_rank: Set(player.squadron_rank.map(|r| format!("{:?}", r))),
        active_ship_id: Set(Some(player.active_ship_id.to_string())),
        faction_id: Set(player.faction_id.to_string()),
        patrol_sector_id: Set(Some(player.patrol_sector_id.to_string())),
        is_online: Set(if player.is_online { 1 } else { 0 }),
        last_seen: Set(Some(player.last_seen.to_rfc3339())),
        offline_attacks_remaining: Set(player.offline_attacks_remaining),
        missions_completed: Set(player.missions_completed),
        missions_failed: Set(player.missions_failed),
        created_at: Set(player.created_at.to_rfc3339()),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
    };

    // Try insert, on conflict update
    player::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(player::Column::Id)
                .update_columns([
                    player::Column::Reputation,
                    player::Column::Fame,
                    player::Column::FactionStandings,
                    player::Column::Stats,
                    player::Column::SquadronId,
                    player::Column::SquadronRank,
                    player::Column::ActiveShipId,
                    player::Column::FactionId,
                    player::Column::PatrolSectorId,
                    player::Column::IsOnline,
                    player::Column::LastSeen,
                    player::Column::OfflineAttacksRemaining,
                    player::Column::MissionsCompleted,
                    player::Column::MissionsFailed,
                    player::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

async fn upsert_ship(db: &DatabaseConnection, ship: &Ship) -> Result<(), sea_orm::DbErr> {
    use entities::ship;
    use sea_orm::EntityTrait;

    let (status, status_data) = serialize_ship_status(&ship.status);

    let model = ship::ActiveModel {
        id: Set(ship.id.to_string()),
        owner_id: Set(ship.owner_id.map(|id| id.to_string())),
        name: Set(ship.name.clone()),
        ship_class: Set(format!("{:?}", ship.ship_class)),
        sector_id: Set(Some(ship.sector_id.to_string())),
        position_x: Set(ship.position.x),
        position_y: Set(ship.position.y),
        position_z: Set(ship.position.z),
        hull_integrity: Set(ship.hull_integrity),
        shield_strength: Set(ship.shield_strength),
        ammunition: Set(ship.resources.ammunition),
        fuel: Set(ship.resources.fuel),
        morale: Set(ship.crew.morale),
        experience: Set(ship.crew.experience),
        weapons: Set(serde_json::to_string(&ship.weapons).unwrap_or_default()),
        status: Set(status),
        status_data: Set(status_data),
        is_player_ship: Set(if ship.is_player_ship { 1 } else { 0 }),
        faction_id: Set(ship.faction_id.map(|id| id.to_string())),
        squadron_id: Set(ship.squadron_id.map(|id| id.to_string())),
        created_at: Set(chrono::Utc::now().to_rfc3339()),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
    };

    ship::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(ship::Column::Id)
                .update_columns([
                    ship::Column::OwnerId,
                    ship::Column::Name,
                    ship::Column::ShipClass,
                    ship::Column::SectorId,
                    ship::Column::PositionX,
                    ship::Column::PositionY,
                    ship::Column::PositionZ,
                    ship::Column::HullIntegrity,
                    ship::Column::ShieldStrength,
                    ship::Column::Ammunition,
                    ship::Column::Fuel,
                    ship::Column::Morale,
                    ship::Column::Experience,
                    ship::Column::Weapons,
                    ship::Column::Status,
                    ship::Column::StatusData,
                    ship::Column::IsPlayerShip,
                    ship::Column::FactionId,
                    ship::Column::SquadronId,
                    ship::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

async fn delete_ship(db: &DatabaseConnection, id: Uuid) -> Result<(), sea_orm::DbErr> {
    use entities::ship;
    use sea_orm::EntityTrait;

    ship::Entity::delete_by_id(id.to_string()).exec(db).await?;
    Ok(())
}

async fn upsert_squadron(db: &DatabaseConnection, squadron: &Squadron) -> Result<(), sea_orm::DbErr> {
    use entities::squadron;
    use sea_orm::EntityTrait;

    let model = squadron::ActiveModel {
        id: Set(squadron.id.to_string()),
        name: Set(squadron.name.clone()),
        tag: Set(squadron.tag.clone()),
        motto: Set(squadron.motto.clone()),
        description: Set(Some(squadron.description.clone())),
        leader_id: Set(Some(squadron.leader_id.to_string())),
        officers: Set(uuid_vec_to_json(&squadron.officers)),
        members: Set(uuid_vec_to_json(&squadron.members)),
        patrol_sectors: Set(uuid_vec_to_json(&squadron.patrol_sectors)),
        owned_stations: Set(uuid_vec_to_json(&squadron.owned_stations)),
        owned_ships: Set(uuid_vec_to_json(&squadron.owned_ships)),
        treasury: Set(squadron.treasury),
        reputation_bonus: Set(squadron.reputation_bonus as f64),
        fame_bonus: Set(squadron.fame_bonus as f64),
        allied_squadrons: Set(uuid_vec_to_json(&squadron.allied_squadrons)),
        hostile_squadrons: Set(uuid_vec_to_json(&squadron.hostile_squadrons)),
        wargames_enabled: Set(if squadron.wargames_enabled { 1 } else { 0 }),
        privateering_enabled: Set(if squadron.privateering_enabled { 1 } else { 0 }),
        settings: Set(serde_json::to_string(&squadron.settings).unwrap_or_default()),
        stats: Set(serde_json::to_string(&squadron.stats).unwrap_or_default()),
        founded_at: Set(squadron.founded_at.to_rfc3339()),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
    };

    squadron::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(squadron::Column::Id)
                .update_columns([
                    squadron::Column::Name,
                    squadron::Column::Tag,
                    squadron::Column::Motto,
                    squadron::Column::Description,
                    squadron::Column::LeaderId,
                    squadron::Column::Officers,
                    squadron::Column::Members,
                    squadron::Column::PatrolSectors,
                    squadron::Column::OwnedStations,
                    squadron::Column::OwnedShips,
                    squadron::Column::Treasury,
                    squadron::Column::ReputationBonus,
                    squadron::Column::FameBonus,
                    squadron::Column::AlliedSquadrons,
                    squadron::Column::HostileSquadrons,
                    squadron::Column::WargamesEnabled,
                    squadron::Column::PrivateeringEnabled,
                    squadron::Column::Settings,
                    squadron::Column::Stats,
                    squadron::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

async fn upsert_mission(db: &DatabaseConnection, mission: &Mission) -> Result<(), sea_orm::DbErr> {
    use entities::mission;
    use sea_orm::EntityTrait;

    let (availability, availability_data) = serialize_availability(&mission.availability);
    let status = serialize_mission_status(&mission.status);

    let model = mission::ActiveModel {
        id: Set(mission.id.to_string()),
        mission_type: Set(format!("{:?}", mission.mission_type)),
        title: Set(mission.title.clone()),
        description: Set(Some(mission.description.clone())),
        script_path: Set(mission.script_path.clone()),
        current_state: Set(mission.current_state.clone()),
        data: Set(mission.data.to_string()),
        sector_id: Set(mission.sector_id.to_string()),
        target_position_x: Set(mission.target_position.map(|p| p.x)),
        target_position_y: Set(mission.target_position.map(|p| p.y)),
        target_position_z: Set(mission.target_position.map(|p| p.z)),
        target_id: Set(mission.target_id.map(|id| id.to_string())),
        assigned_to: Set(mission.assigned_to.map(|id| id.to_string())),
        availability: Set(availability),
        availability_data: Set(availability_data),
        reputation_reward: Set(mission.reputation_reward),
        reputation_penalty: Set(mission.reputation_penalty),
        fame_reward: Set(mission.fame_reward),
        credits_reward: Set(mission.credits_reward as i32),
        status: Set(status),
        progress: Set(mission.progress as f64),
        is_high_profile: Set(if mission.is_high_profile { 1 } else { 0 }),
        priority: Set(format!("{:?}", mission.priority)),
        expires_at: Set(mission.expires_at.map(|dt| dt.to_rfc3339())),
        created_at: Set(mission.created_at.to_rfc3339()),
        updated_at: Set(chrono::Utc::now().to_rfc3339()),
    };

    mission::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::column(mission::Column::Id)
                .update_columns([
                    mission::Column::CurrentState,
                    mission::Column::Data,
                    mission::Column::TargetPositionX,
                    mission::Column::TargetPositionY,
                    mission::Column::TargetPositionZ,
                    mission::Column::TargetId,
                    mission::Column::AssignedTo,
                    mission::Column::Availability,
                    mission::Column::AvailabilityData,
                    mission::Column::Status,
                    mission::Column::Progress,
                    mission::Column::IsHighProfile,
                    mission::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

// ============================================================================
// Serialization helpers (matching existing converters)
// ============================================================================

use bw_core::models::{MissionAvailability, MissionStatus, ShipStatus};

fn serialize_ship_status(status: &ShipStatus) -> (String, String) {
    match status {
        ShipStatus::Idle => ("idle".to_string(), "{}".to_string()),
        ShipStatus::InTransit { destination, target_id } => {
            let data = serde_json::json!({
                "destination_x": destination.x,
                "destination_y": destination.y,
                "destination_z": destination.z,
                "target_id": target_id.map(|id| id.to_string()),
            });
            ("in_transit".to_string(), data.to_string())
        }
        ShipStatus::InCombat { engagement_id } => {
            let data = serde_json::json!({
                "engagement_id": engagement_id.to_string(),
            });
            ("in_combat".to_string(), data.to_string())
        }
        ShipStatus::Docked { station_id } => {
            let data = serde_json::json!({
                "station_id": station_id.to_string(),
            });
            ("docked".to_string(), data.to_string())
        }
        ShipStatus::Disabled => ("disabled".to_string(), "{}".to_string()),
        ShipStatus::Destroyed => ("destroyed".to_string(), "{}".to_string()),
    }
}

fn serialize_availability(availability: &MissionAvailability) -> (String, Option<String>) {
    match availability {
        MissionAvailability::SectorWide => ("sector_wide".to_string(), None),
        MissionAvailability::RangeRestricted(range) => {
            ("range_restricted".to_string(), Some(range.to_string()))
        }
        MissionAvailability::Assigned(player_id) => {
            ("assigned".to_string(), Some(player_id.to_string()))
        }
        MissionAvailability::SquadronOnly(squadron_id) => {
            ("squadron_only".to_string(), Some(squadron_id.to_string()))
        }
    }
}

fn serialize_mission_status(status: &MissionStatus) -> String {
    match status {
        MissionStatus::Available => "available".to_string(),
        MissionStatus::InProgress => "in_progress".to_string(),
        MissionStatus::Completed { success: true } => "completed_success".to_string(),
        MissionStatus::Completed { success: false } => "completed_failure".to_string(),
        MissionStatus::Expired => "expired".to_string(),
        MissionStatus::Abandoned => "abandoned".to_string(),
    }
}

fn uuid_vec_to_json(uuids: &[Uuid]) -> String {
    let strings: Vec<String> = uuids.iter().map(|u| u.to_string()).collect();
    serde_json::to_string(&strings).unwrap_or_else(|_| "[]".to_string())
}
