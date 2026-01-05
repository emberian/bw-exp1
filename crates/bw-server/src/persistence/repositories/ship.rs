//! Ship repository - CRUD operations for ships.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::Ship;

use crate::persistence::{converters, models::ShipRow, DbError};

/// Repository for ship operations.
pub struct ShipRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> ShipRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a ship by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Ship>, DbError> {
        let row = sqlx::query_as::<_, ShipRow>("SELECT * FROM ships WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::ship_from_row))
    }

    /// Find all ships owned by a player.
    pub async fn find_by_owner(&self, owner_id: Uuid) -> Result<Vec<Ship>, DbError> {
        let rows = sqlx::query_as::<_, ShipRow>("SELECT * FROM ships WHERE owner_id = ?")
            .bind(owner_id.to_string())
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(converters::ship_from_row).collect())
    }

    /// Find all ships in a sector.
    pub async fn find_by_sector(&self, sector_id: Uuid) -> Result<Vec<Ship>, DbError> {
        let rows = sqlx::query_as::<_, ShipRow>("SELECT * FROM ships WHERE sector_id = ?")
            .bind(sector_id.to_string())
            .fetch_all(self.pool)
            .await?;

        Ok(rows.into_iter().map(converters::ship_from_row).collect())
    }

    /// Find all player ships in a sector.
    pub async fn find_player_ships_in_sector(&self, sector_id: Uuid) -> Result<Vec<Ship>, DbError> {
        let rows = sqlx::query_as::<_, ShipRow>(
            "SELECT * FROM ships WHERE sector_id = ? AND is_player_ship = 1",
        )
        .bind(sector_id.to_string())
        .fetch_all(self.pool)
        .await?;

        Ok(rows.into_iter().map(converters::ship_from_row).collect())
    }

    /// Insert a new ship.
    pub async fn insert(&self, ship: &Ship) -> Result<(), DbError> {
        let params = converters::ship_to_insert_params(ship);

        sqlx::query(
            r#"
            INSERT INTO ships (
                id, owner_id, name, ship_class, sector_id,
                position_x, position_y, position_z,
                hull_integrity, shield_strength, ammunition, fuel,
                morale, experience, weapons, status, status_data,
                is_player_ship, faction_id, squadron_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&params.id)
        .bind(&params.owner_id)
        .bind(&params.name)
        .bind(&params.ship_class)
        .bind(&params.sector_id)
        .bind(params.position_x)
        .bind(params.position_y)
        .bind(params.position_z)
        .bind(params.hull_integrity)
        .bind(params.shield_strength)
        .bind(params.ammunition)
        .bind(params.fuel)
        .bind(params.morale)
        .bind(params.experience)
        .bind(&params.weapons)
        .bind(&params.status)
        .bind(&params.status_data)
        .bind(params.is_player_ship)
        .bind(&params.faction_id)
        .bind(&params.squadron_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update a ship.
    pub async fn update(&self, ship: &Ship) -> Result<(), DbError> {
        let params = converters::ship_to_insert_params(ship);

        sqlx::query(
            r#"
            UPDATE ships SET
                owner_id = ?, name = ?, ship_class = ?, sector_id = ?,
                position_x = ?, position_y = ?, position_z = ?,
                hull_integrity = ?, shield_strength = ?, ammunition = ?, fuel = ?,
                morale = ?, experience = ?, weapons = ?, status = ?, status_data = ?,
                is_player_ship = ?, faction_id = ?, squadron_id = ?
            WHERE id = ?
            "#,
        )
        .bind(&params.owner_id)
        .bind(&params.name)
        .bind(&params.ship_class)
        .bind(&params.sector_id)
        .bind(params.position_x)
        .bind(params.position_y)
        .bind(params.position_z)
        .bind(params.hull_integrity)
        .bind(params.shield_strength)
        .bind(params.ammunition)
        .bind(params.fuel)
        .bind(params.morale)
        .bind(params.experience)
        .bind(&params.weapons)
        .bind(&params.status)
        .bind(&params.status_data)
        .bind(params.is_player_ship)
        .bind(&params.faction_id)
        .bind(&params.squadron_id)
        .bind(&params.id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update only position (for frequent updates).
    pub async fn update_position(
        &self,
        ship_id: Uuid,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<(), DbError> {
        sqlx::query(
            "UPDATE ships SET position_x = ?, position_y = ?, position_z = ? WHERE id = ?",
        )
        .bind(x)
        .bind(y)
        .bind(z)
        .bind(ship_id.to_string())
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Update only resources (for frequent updates).
    pub async fn update_resources(
        &self,
        ship_id: Uuid,
        ammunition: f32,
        fuel: f32,
        morale: f32,
        hull: f32,
        shields: f32,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE ships SET
                ammunition = ?, fuel = ?, morale = ?,
                hull_integrity = ?, shield_strength = ?
            WHERE id = ?
            "#,
        )
        .bind(ammunition)
        .bind(fuel)
        .bind(morale)
        .bind(hull)
        .bind(shields)
        .bind(ship_id.to_string())
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// Delete a ship.
    pub async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let result = sqlx::query("DELETE FROM ships WHERE id = ?")
            .bind(id.to_string())
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
