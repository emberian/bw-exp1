//! Mission choice entity (logs player choices during missions).

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "mission_choices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub mission_id: String,
    pub player_id: String,
    pub choice_id: String,
    pub choice_label: Option<String>,
    pub outcome: Option<String>, // JSON
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::mission::Entity",
        from = "Column::MissionId",
        to = "super::mission::Column::Id"
    )]
    Mission,
    #[sea_orm(
        belongs_to = "super::player::Entity",
        from = "Column::PlayerId",
        to = "super::player::Column::Id"
    )]
    Player,
}

impl Related<super::mission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Mission.def()
    }
}

impl Related<super::player::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Player.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
