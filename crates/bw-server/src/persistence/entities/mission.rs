//! Mission entity.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "missions")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub mission_type: String,
    pub title: String,
    pub description: Option<String>,
    pub script_path: String,
    pub current_state: String,
    pub data: String, // JSON
    pub sector_id: String,
    pub target_position_x: Option<f64>,
    pub target_position_y: Option<f64>,
    pub target_position_z: Option<f64>,
    pub target_id: Option<String>,
    pub assigned_to: Option<String>,
    pub availability: String,
    pub availability_data: Option<String>,
    pub reputation_reward: i32,
    pub reputation_penalty: i32,
    pub fame_reward: i32,
    pub credits_reward: i32,
    pub status: String,
    pub progress: f64,
    pub is_high_profile: i32,
    pub priority: String,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
        belongs_to = "super::player::Entity",
        from = "Column::AssignedTo",
        to = "super::player::Column::Id"
    )]
    AssignedPlayer,
    #[sea_orm(has_many = "super::mission_choice::Entity")]
    Choices,
}

impl Related<super::sector::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sector.def()
    }
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AssignedPlayer.def()
    }
}

impl Related<super::mission_choice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Choices.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
