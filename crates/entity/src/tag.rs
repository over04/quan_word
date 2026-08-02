use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tag")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "wordbook_id")]
    pub wordbook_id: i32,
    pub name: String,
    pub created_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "wordbook_id", to = "id")]
    pub wordbook: BelongsTo<super::wordbook::Entity>,
    #[sea_orm(has_many)]
    pub word_tags: HasMany<super::word_tag::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
