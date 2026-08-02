use std::collections::HashMap;

use entity::tag;
use entity::word_tag;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};

use super::dto::resp::TagResp;

/// 标签持久化访问：SeaORM 查询封装。
pub struct TagRepo;

impl TagRepo {
    /// 某本书的全部标签（按创建时间升序）及各自单词数。
    pub async fn find_all_with_counts(
        db: &DatabaseConnection,
        book_id: i32,
    ) -> Result<Vec<TagResp>, sea_orm::DbErr> {
        let tags = tag::Entity::find()
            .filter(tag::Column::WordbookId.eq(book_id))
            .order_by_asc(tag::Column::CreatedAt)
            .all(db)
            .await?;
        let counts: HashMap<i32, u64> = word_tag::Entity::find()
            .select_only()
            .column(word_tag::Column::TagId)
            .column_as(
                Expr::col((word_tag::Entity, word_tag::Column::WordId)).count(),
                "c",
            )
            .group_by(word_tag::Column::TagId)
            .into_tuple::<(i32, i64)>()
            .all(db)
            .await?
            .into_iter()
            .map(|(tid, c)| (tid, c as u64))
            .collect();
        Ok(tags
            .into_iter()
            .map(|t| TagResp {
                id: t.id,
                name: t.name,
                word_count: counts.get(&t.id).copied().unwrap_or(0),
            })
            .collect())
    }

    /// 单个标签的单词数。
    pub async fn count_word(db: &DatabaseConnection, tag_id: i32) -> Result<u64, sea_orm::DbErr> {
        word_tag::Entity::find()
            .filter(word_tag::Column::TagId.eq(tag_id))
            .count(db)
            .await
    }

    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<tag::Model>, sea_orm::DbErr> {
        tag::Entity::find_by_id(id).one(db).await
    }

    /// 同书同名标签（重名检查）；`exclude_id` 用于更新时排除自身。
    pub async fn find_dup(
        db: &DatabaseConnection,
        book_id: i32,
        name: &str,
        exclude_id: Option<i32>,
    ) -> Result<Option<tag::Model>, sea_orm::DbErr> {
        let mut q = tag::Entity::find()
            .filter(tag::Column::WordbookId.eq(book_id))
            .filter(tag::Column::Name.eq(name));
        if let Some(id) = exclude_id {
            q = q.filter(tag::Column::Id.ne(id));
        }
        q.one(db).await
    }

    pub async fn insert(
        db: &DatabaseConnection,
        model: tag::ActiveModel,
    ) -> Result<tag::Model, sea_orm::DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: tag::ActiveModel,
    ) -> Result<tag::Model, sea_orm::DbErr> {
        model.update(db).await
    }

    /// 删除标签；返回受影响行数（0 = 不存在）。
    pub async fn delete_by_id(db: &DatabaseConnection, id: i32) -> Result<u64, sea_orm::DbErr> {
        Ok(tag::Entity::delete_by_id(id).exec(db).await?.rows_affected)
    }
}
