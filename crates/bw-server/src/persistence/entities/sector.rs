//! Sector entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sectors")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub name: String,
    pub description: Option<String>,
    pub bounds_min_x: f64,
    pub bounds_min_y: f64,
    pub bounds_min_z: f64,
    pub bounds_max_x: f64,
    pub bounds_max_y: f64,
    pub bounds_max_z: f64,
    pub danger_level: String,
    pub traffic_density: String,
    pub fuel_cost_modifier: f32,
    pub is_core_sector: i32,
    pub controlling_faction_id: Option<String>,
    pub controlling_squadron_id: Option<String>,
    pub adjacent_sectors: String, // JSON array
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::location::Entity")]
    Locations,
    #[sea_orm(has_many = "super::ship::Entity")]
    Ships,
    #[sea_orm(has_many = "super::mission::Entity")]
    Missions,
}

impl Related<super::location::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Locations.def()
    }
}

impl Related<super::ship::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ships.def()
    }
}

impl Related<super::mission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Missions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
