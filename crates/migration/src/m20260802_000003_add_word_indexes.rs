use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 本文件自包含定义（旧迁移中的 Word 枚举为私有，无法跨文件引用）。
#[derive(Iden)]
enum Word {
    Table,
    WordbookId,
    Spelling,
    CreatedAt,
    UpdatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 纸质书 spelling 排序分页
        manager
            .create_index(
                Index::create()
                    .name("idx-word-book-spelling")
                    .table(Word::Table)
                    .col(Word::WordbookId)
                    .col(Word::Spelling)
                    .to_owned(),
            )
            .await?;
        // 列表模式默认排序（created_at）
        manager
            .create_index(
                Index::create()
                    .name("idx-word-book-created")
                    .table(Word::Table)
                    .col(Word::WordbookId)
                    .col(Word::CreatedAt)
                    .to_owned(),
            )
            .await?;
        // 列表模式 updated_at 排序
        manager
            .create_index(
                Index::create()
                    .name("idx-word-book-updated")
                    .table(Word::Table)
                    .col(Word::WordbookId)
                    .col(Word::UpdatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx-word-book-spelling")
                    .table(Word::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx-word-book-created")
                    .table(Word::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_index(
                Index::drop()
                    .name("idx-word-book-updated")
                    .table(Word::Table)
                    .to_owned(),
            )
            .await
    }
}
