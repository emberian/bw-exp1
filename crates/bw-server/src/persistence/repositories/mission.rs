//! Mission repository - CRUD operations for missions.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::Mission;

use crate::persistence::{converters, models::MissionRow, DbError};

/// Repository for mission operations.
pub struct MissionRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> MissionRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a mission by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Mission>, DbError> {
        let row = sqlx::query_as::<_, MissionRow>("SELECT * FROM missions WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::mission_from_row))
    }

    /// Find all missions in a sector.
    pub async fn find_by_sector(&self, sector_id: Uuid) -> Result<Vec<Mission>, DbError> {
        let rows = sqlx::query_as::<_, MissionRow>("SELECT * FROM missions WHERE sector_id = ?")
            .bind(sector_id.to_string())
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(converters::mission_from_row)
            .collect())
    }

    /// Find available missions in a sector.
    pub async fn find_available_in_sector(&self, sector_id: Uuid) -> Result<Vec<Mission>, DbError> {
        let rows = sqlx::query_as::<_, MissionRow>(
            "SELECT * FROM missions WHERE sector_id = ? AND status = 'available' ORDER BY priority DESC, created_at ASC",
        )
        .bind(sector_id.to_string())
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(converters::mission_from_row)
            .collect())
    }

    /// Find missions assigned to a player.
    pub async fn find_by_player(&self, player_id: Uuid) -> Result<Vec<Mission>, DbError> {
        let rows = sqlx::query_as::<_, MissionRow>("SELECT * FROM missions WHERE assigned_to = ?")
            .bind(player_id.to_string())
            .fetch_all(self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(converters::mission_from_row)
            .collect())
    }

    /// Find active missions for a player (in progress).
    pub async fn find_active_for_player(&self, player_id: Uuid) -> Result<Vec<Mission>, DbError> {
        let rows = sqlx::query_as::<_, MissionRow>(
            "SELECT * FROM missions WHERE assigned_to = ? AND status = 'in_progress'",
        )
        .bind(player_id.to_string())
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(converters::mission_from_row)
            .collect())
    }

    /// Insert a new mission.
    pub async fn insert(&self, mission: &Mission) -> Result<(), DbError> {
        let (availability, availability_data) = serialize_availability(&mission.availability);
        let status = serialize_status(&mission.status);

        sqlx::query(
            r#"
            INSERT INTO missions (
                id, mission_type, title, description, script_path,
                current_state, data, sector_id,
                target_position_x, target_position_y, target_position_z, target_id,
                assigned_to, availability, availability_data,
                reputation_reward, reputation_penalty, fame_reward, credits_reward,
                status, progress, is_high_profile, priority, expires_at, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(mission.id.to_string())
        .bind(format!("{:?}", mission.mission_type))
        .bind(&mission.title)
        .bind(&mission.description)
        .bind(&mission.script_path)
        .bind(&mission.current_state)
        .bind(mission.data.to_string())
        .bind(mission.sector_id.to_string())
        .bind(mission.target_position.map(|p| p.x))
        .bind(mission.target_position.map(|p| p.y))
        .bind(mission.target_position.map(|p| p.z))
        .bind(mission.target_id.map(|id| id.to_string()))
        .bind(mission.assigned_to.map(|id| id.to_string()))
        .bind(&availability)
        .bind(&availability_data)
        .bind(mission.reputation_reward)
        .bind(mission.reputation_penalty)
        .bind(mission.fame_reward)
        .bind(mission.credits_reward)
        .bind(&status)
        .bind(mission.progress)
        .bind(if mission.is_high_profile { 1 } else { 0 })
        .bind(format!("{:?}", mission.priority))
        .bind(mission.expires_at.map(|dt| dt.to_rfc3339()))
        .bind(mission.created_at.to_rfc3339())
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update a mission.
    pub async fn update(&self, mission: &Mission) -> Result<(), DbError> {
        let (availability, availability_data) = serialize_availability(&mission.availability);
        let status = serialize_status(&mission.status);

        sqlx::query(
            r#"
            UPDATE missions SET
                current_state = ?, data = ?,
                target_position_x = ?, target_position_y = ?, target_position_z = ?,
                assigned_to = ?, availability = ?, availability_data = ?,
                status = ?, progress = ?, is_high_profile = ?
            WHERE id = ?
            "#,
        )
        .bind(&mission.current_state)
        .bind(mission.data.to_string())
        .bind(mission.target_position.map(|p| p.x))
        .bind(mission.target_position.map(|p| p.y))
        .bind(mission.target_position.map(|p| p.z))
        .bind(mission.assigned_to.map(|id| id.to_string()))
        .bind(&availability)
        .bind(&availability_data)
        .bind(&status)
        .bind(mission.progress)
        .bind(if mission.is_high_profile { 1 } else { 0 })
        .bind(mission.id.to_string())
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a mission.
    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM missions WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete expired missions.
    pub async fn delete_expired(&self) -> Result<u64, DbError> {
        let result = sqlx::query(
            "DELETE FROM missions WHERE expires_at IS NOT NULL AND expires_at < datetime('now') AND status = 'available'",
        )
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

use bw_core::models::{MissionAvailability, MissionStatus};

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

fn serialize_status(status: &MissionStatus) -> String {
    match status {
        MissionStatus::Available => "available".to_string(),
        MissionStatus::InProgress => "in_progress".to_string(),
        MissionStatus::Completed { success: true } => "completed_success".to_string(),
        MissionStatus::Completed { success: false } => "completed_failure".to_string(),
        MissionStatus::Expired => "expired".to_string(),
        MissionStatus::Abandoned => "abandoned".to_string(),
    }
}
