//! Squadron entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "squadrons")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub tag: String,
    pub motto: Option<String>,
    pub description: Option<String>,
    pub leader_id: Option<String>,
    pub officers: String,       // JSON array
    pub members: String,        // JSON array
    pub patrol_sectors: String, // JSON array
    pub owned_stations: String, // JSON array
    pub owned_ships: String,    // JSON array
    pub treasury: i64,
    pub reputation_bonus: f64,
    pub fame_bonus: f64,
    pub allied_squadrons: String,  // JSON array
    pub hostile_squadrons: String, // JSON array
    pub wargames_enabled: i32,
    pub privateering_enabled: i32,
    pub settings: String, // JSON
    pub stats: String,    // JSON
    pub founded_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::player::Entity")]
    Members,
    #[sea_orm(has_many = "super::ship::Entity")]
    Ships,
    #[sea_orm(
        belongs_to = "super::player::Entity",
        from = "Column::LeaderId",
        to = "super::player::Column::Id"
    )]
    Leader,
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}

impl Related<super::ship::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ships.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
