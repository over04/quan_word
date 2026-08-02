use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "word")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "wordbook_id")]
    pub wordbook_id: i32,
    pub spelling: String,
    pub phonetic: Option<String>,
    #[sea_orm(column_type = "Json")]
    pub definitions: serde_json::Value,
    pub example: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "wordbook_id", to = "id")]
    pub wordbook: BelongsTo<super::wordbook::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
