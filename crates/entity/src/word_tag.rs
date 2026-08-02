use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "word_tag")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub word_id: i32,
    #[sea_orm(primary_key)]
    pub tag_id: i32,
    #[sea_orm(belongs_to, from = "word_id", to = "id")]
    pub word: BelongsTo<super::word::Entity>,
    #[sea_orm(belongs_to, from = "tag_id", to = "id")]
    pub tag: BelongsTo<super::tag::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
