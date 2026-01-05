//! Ship entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "ships")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub owner_id: Option<String>,
    pub name: String,
    pub ship_class: String,
    pub sector_id: Option<String>,
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub hull_integrity: f32,
    pub shield_strength: f32,
    pub ammunition: f32,
    pub fuel: f32,
    pub morale: f32,
    pub experience: i32,
    pub weapons: String, // JSON
    pub status: String,
    pub status_data: String, // JSON
    pub is_player_ship: i32,
    pub faction_id: Option<String>,
    pub squadron_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::player::Entity",
        from = "Column::OwnerId",
        to = "super::player::Column::Id"
    )]
    Owner,
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
    #[sea_orm(
        belongs_to = "super::squadron::Entity",
        from = "Column::SquadronId",
        to = "super::squadron::Column::Id"
    )]
    Squadron,
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<super::sector::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sector.def()
    }
}

impl Related<super::faction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Faction.def()
    }
}

impl Related<super::squadron::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Squadron.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
