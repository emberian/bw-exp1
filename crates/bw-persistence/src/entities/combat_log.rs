//! Combat log entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "combat_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub sector_id: Option<String>,
    pub attackers: String,    // JSON
    pub defenders: String,    // JSON
    pub winner: Option<String>,
    pub outcome_data: String, // JSON
    pub events: String,       // JSON array
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sector::Entity",
        from = "Column::SectorId",
        to = "super::sector::Column::Id"
    )]
    Sector,
}

impl Related<super::sector::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sector.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
