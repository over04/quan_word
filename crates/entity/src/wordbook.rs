use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "wordbook")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    #[sea_orm(default_value = "")]
    pub description: String,
    #[sea_orm(default_value = "📖")]
    pub icon: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(has_many)]
    pub words: HasMany<super::word::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
