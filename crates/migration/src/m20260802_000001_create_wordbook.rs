use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
pub enum Wordbook {
    Table,
    Id,
    Name,
    Description,
    Icon,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Wordbook::Table)
                    .if_not_exists()
                    .col(pk_auto(Wordbook::Id))
                    .col(string(Wordbook::Name))
                    .col(string(Wordbook::Description).default(""))
                    .col(string(Wordbook::Icon).default("📖"))
                    .col(timestamp_with_time_zone(Wordbook::CreatedAt))
                    .col(timestamp_with_time_zone(Wordbook::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Wordbook::Table).to_owned())
            .await
    }
}
