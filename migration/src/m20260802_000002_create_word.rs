use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260802_000001_create_wordbook::Wordbook;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Iden)]
enum Word {
    Table,
    Id,
    WordbookId,
    Spelling,
    Phonetic,
    Definitions,
    Example,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Word::Table)
                    .if_not_exists()
                    .col(pk_auto(Word::Id))
                    .col(integer(Word::WordbookId))
                    .col(string(Word::Spelling))
                    .col(string_null(Word::Phonetic))
                    .col(json(Word::Definitions))
                    .col(string_null(Word::Example))
                    .col(timestamp_with_time_zone(Word::CreatedAt))
                    .col(timestamp_with_time_zone(Word::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-word-wordbook_id")
                            .from(Word::Table, Word::WordbookId)
                            .to(Wordbook::Table, Wordbook::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-word-wordbook_id")
                    .table(Word::Table)
                    .col(Word::WordbookId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Word::Table).to_owned())
            .await
    }
}
