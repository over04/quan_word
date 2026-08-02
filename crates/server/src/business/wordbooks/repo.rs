use std::collections::HashMap;

use entity::{word, wordbook};
use sea_orm::sea_query::{Expr, Func};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};

/// 单词书持久化访问：SeaORM 查询封装。
pub struct WordbookRepo;

impl WordbookRepo {
    pub async fn find_all(db: &DatabaseConnection) -> Result<Vec<wordbook::Model>, sea_orm::DbErr> {
        wordbook::Entity::find()
            .order_by_asc(wordbook::Column::Id)
            .all(db)
            .await
    }

    /// 各单词书的单词数：单次 GROUP BY 聚合，避免每本书一次 count（N+1）。
    pub async fn count_words_by_book(
        db: &DatabaseConnection,
    ) -> Result<HashMap<i32, i64>, sea_orm::DbErr> {
        let rows: Vec<(i32, i64)> = word::Entity::find()
            .select_only()
            .column(word::Column::WordbookId)
            .column_as(Expr::expr(Func::count(Expr::col(word::Column::Id))), "cnt")
            .group_by(word::Column::WordbookId)
            .into_tuple()
            .all(db)
            .await?;
        Ok(rows.into_iter().collect())
    }

    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<wordbook::Model>, sea_orm::DbErr> {
        wordbook::Entity::find_by_id(id).one(db).await
    }

    pub async fn insert(
        db: &DatabaseConnection,
        model: wordbook::ActiveModel,
    ) -> Result<wordbook::Model, sea_orm::DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: wordbook::ActiveModel,
    ) -> Result<wordbook::Model, sea_orm::DbErr> {
        model.update(db).await
    }

    /// 删除单词书；返回受影响行数（0 = 不存在）。
    pub async fn delete_by_id(db: &DatabaseConnection, id: i32) -> Result<u64, sea_orm::DbErr> {
        Ok(wordbook::Entity::delete_by_id(id)
            .exec(db)
            .await?
            .rows_affected)
    }

    pub async fn count_words(db: &DatabaseConnection, book_id: i32) -> Result<u64, sea_orm::DbErr> {
        word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .count(db)
            .await
    }
}
