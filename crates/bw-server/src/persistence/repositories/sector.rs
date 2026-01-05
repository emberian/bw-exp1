//! Sector repository - access to sectors and locations.

use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use bw_core::models::{Location, Sector};

use crate::persistence::{
    converters,
    models::{LocationRow, SectorRow},
    DbError,
};

/// Repository for sector operations.
pub struct SectorRepository<'a> {
    pool: &'a Pool<Sqlite>,
}

impl<'a> SectorRepository<'a> {
    pub fn new(pool: &'a Pool<Sqlite>) -> Self {
        Self { pool }
    }

    /// Find a sector by ID.
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Sector>, DbError> {
        let row = sqlx::query_as::<_, SectorRow>("SELECT * FROM sectors WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        match row {
            Some(row) => {
                let mut sector = converters::sector_from_row(row);
                sector.locations = self.find_locations_for_sector(id).await?;
                Ok(Some(sector))
            }
            None => Ok(None),
        }
    }

    /// Find a sector by name (case-insensitive).
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Sector>, DbError> {
        let row =
            sqlx::query_as::<_, SectorRow>("SELECT * FROM sectors WHERE name = ? COLLATE NOCASE")
                .bind(name)
                .fetch_optional(self.pool)
                .await?;

        match row {
            Some(row) => {
                let sector_id = Uuid::parse_str(&row.id)?;
                let mut sector = converters::sector_from_row(row);
                sector.locations = self.find_locations_for_sector(sector_id).await?;
                Ok(Some(sector))
            }
            None => Ok(None),
        }
    }

    /// Get all sectors.
    pub async fn find_all(&self) -> Result<Vec<Sector>, DbError> {
        let rows = sqlx::query_as::<_, SectorRow>("SELECT * FROM sectors ORDER BY name")
            .fetch_all(self.pool)
            .await?;

        let mut sectors = Vec::with_capacity(rows.len());
        for row in rows {
            let sector_id = Uuid::parse_str(&row.id)?;
            let mut sector = converters::sector_from_row(row);
            sector.locations = self.find_locations_for_sector(sector_id).await?;
            sectors.push(sector);
        }

        Ok(sectors)
    }

    /// Get all sectors with their locations loaded.
    pub async fn find_all_with_locations(&self) -> Result<Vec<Sector>, DbError> {
        self.find_all().await
    }

    /// Find locations for a sector.
    pub async fn find_locations_for_sector(
        &self,
        sector_id: Uuid,
    ) -> Result<Vec<Location>, DbError> {
        let rows = sqlx::query_as::<_, LocationRow>(
            "SELECT * FROM locations WHERE sector_id = ? ORDER BY name",
        )
        .bind(sector_id.to_string())
        .fetch_all(self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(converters::location_from_row)
            .collect())
    }

    /// Find a specific location by ID.
    pub async fn find_location_by_id(&self, id: Uuid) -> Result<Option<Location>, DbError> {
        let row = sqlx::query_as::<_, LocationRow>("SELECT * FROM locations WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(self.pool)
            .await?;

        Ok(row.map(converters::location_from_row))
    }
}
