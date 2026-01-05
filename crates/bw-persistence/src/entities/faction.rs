//! Faction entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "factions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[sea_orm(unique)]
    pub name: String,
    #[sea_orm(unique)]
    pub tag: String,
    pub faction_type: String,
    pub description: Option<String>,
    pub philosophy: Option<String>,
    pub aesthetic: Option<String>,
    pub is_playable: i32,
    pub is_hostile: i32,
    pub default_standings: String, // JSON
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::player::Entity")]
    Players,
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Players.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
