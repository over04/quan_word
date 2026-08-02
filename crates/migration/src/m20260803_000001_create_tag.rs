use sea_orm_migration::{prelude::*, schema::*};

use crate::m20260802_000001_create_wordbook::Wordbook;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 自包含定义（旧迁移中的 Word 枚举为私有，无法跨文件引用）。
#[derive(Iden)]
enum Word {
    Table,
    Id,
}

#[derive(Iden)]
enum Tag {
    Table,
    Id,
    WordbookId,
    Name,
    CreatedAt,
}

#[derive(Iden)]
enum WordTag {
    Table,
    WordId,
    TagId,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Tag::Table)
                    .if_not_exists()
                    .col(pk_auto(Tag::Id))
                    .col(integer(Tag::WordbookId))
                    .col(string(Tag::Name))
                    .col(timestamp_with_time_zone(Tag::CreatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-tag-wordbook_id")
                            .from(Tag::Table, Tag::WordbookId)
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
                    .name("idx-tag-wordbook_id")
                    .table(Tag::Table)
                    .col(Tag::WordbookId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-tag-wordbook_id-name")
                    .table(Tag::Table)
                    .col(Tag::WordbookId)
                    .col(Tag::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(WordTag::Table)
                    .if_not_exists()
                    .col(integer(WordTag::WordId))
                    .col(integer(WordTag::TagId))
                    .primary_key(Index::create().col(WordTag::WordId).col(WordTag::TagId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-word_tag-word_id")
                            .from(WordTag::Table, WordTag::WordId)
                            .to(Word::Table, Word::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-word_tag-tag_id")
                            .from(WordTag::Table, WordTag::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx-word_tag-tag_id")
                    .table(WordTag::Table)
                    .col(WordTag::TagId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(WordTag::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Tag::Table).to_owned())
            .await
    }
}
