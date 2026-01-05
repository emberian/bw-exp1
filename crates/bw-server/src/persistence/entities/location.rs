//! Location entity (stations, jumpgates, etc within sectors).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "locations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub sector_id: String,
    pub name: String,
    pub description: Option<String>,
    pub location_type: String,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub faction_id: Option<String>,
    pub services: String, // JSON array
    pub is_active: i32,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sector::Entity",
        from = "Column::SectorId",
        to = "super::sector::Column::Id"
    )]
    Sector,
    #[sea_orm(
        belongs_to = "super::faction::Entity",
        from = "Column::FactionId",
        to = "super::faction::Column::Id"
    )]
    Faction,
}

impl Related<super::sector::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sector.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
