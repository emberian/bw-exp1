//! Player entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "players")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub username: String,
    pub password_hash: String,
    pub reputation: i32,
    pub fame: i32,
    pub faction_standings: String, // JSON
    pub stats: String,             // JSON
    pub squadron_id: Option<String>,
    pub squadron_rank: Option<String>,
    pub active_ship_id: Option<String>,
    pub faction_id: String,
    pub patrol_sector_id: Option<String>,
    pub is_online: i32,
    pub last_seen: Option<String>,
    pub offline_attacks_remaining: i32,
    pub missions_completed: i32,
    pub missions_failed: i32,
    pub credits: Option<i64>,
    pub game_mode: Option<String>,
    pub owned_ships: Option<String>, // JSON array of UUIDs
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::ship::Entity")]
    Ships,
    #[sea_orm(has_many = "super::session::Entity")]
    Sessions,
    #[sea_orm(
        belongs_to = "super::squadron::Entity",
        from = "Column::SquadronId",
        to = "super::squadron::Column::Id"
    )]
    Squadron,
    #[sea_orm(
        belongs_to = "super::faction::Entity",
        from = "Column::FactionId",
        to = "super::faction::Column::Id"
    )]
    Faction,
}

impl Related<super::ship::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Ships.def()
    }
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sessions.def()
    }
}

impl Related<super::squadron::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Squadron.def()
    }
}

impl Related<super::faction::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Faction.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
