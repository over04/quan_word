use entity::{word, wordbook};
use sea_orm::sea_query::{Condition, Expr, ExprTrait};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
};

use super::sort::SortField;
use super::sort_dir::SortDir;

/// 单词持久化访问：SeaORM 查询封装。
pub struct WordRepo;

impl WordRepo {
    /// 单词书存在性检查（单词操作的前置校验）。
    pub async fn find_wordbook(
        db: &DatabaseConnection,
        book_id: i32,
    ) -> Result<Option<wordbook::Model>, sea_orm::DbErr> {
        wordbook::Entity::find_by_id(book_id).one(db).await
    }

    /// 浏览模式分页（id / 字母序，SQL 层排序）。
    pub async fn browse_page(
        db: &DatabaseConnection,
        book_id: i32,
        column: word::Column,
        dir: SortDir,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<word::Model>, u64), sea_orm::DbErr> {
        let mut q = word::Entity::find().filter(word::Column::WordbookId.eq(book_id));
        q = match dir {
            SortDir::Asc => q.order_by_asc(column),
            SortDir::Desc => q.order_by_desc(column),
        };
        let paginator = q.paginate(db, page_size);
        let models = paginator.fetch_page(page - 1).await?;
        let total = paginator.num_items().await?;
        Ok((models, total))
    }

    /// 某本书的全部单词 id（供 seeded 洗牌）。
    pub async fn find_all_ids(
        db: &DatabaseConnection,
        book_id: i32,
    ) -> Result<Vec<i32>, sea_orm::DbErr> {
        word::Entity::find()
            .select_only()
            .column(word::Column::Id)
            .filter(word::Column::WordbookId.eq(book_id))
            .into_tuple()
            .all(db)
            .await
    }

    /// 按 id 切片批量取单词（洗牌页查询；结果无序，由调用方按切片顺序重排）。
    pub async fn find_by_ids(
        db: &DatabaseConnection,
        book_id: i32,
        ids: &[i32],
    ) -> Result<Vec<word::Model>, sea_orm::DbErr> {
        word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .filter(word::Column::Id.is_in(ids.iter().copied()))
            .all(db)
            .await
    }

    /// 列表模式查询：书内搜索（拼写/释义模糊匹配）+ 排序 + 分页。
    pub async fn search_page(
        db: &DatabaseConnection,
        book_id: i32,
        q: Option<&str>,
        field: SortField,
        dir: SortDir,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<word::Model>, u64), sea_orm::DbErr> {
        let mut query = word::Entity::find().filter(word::Column::WordbookId.eq(book_id));
        if let Some(q) = q {
            let pat = format!("%{q}%");
            query = query.filter(
                Condition::any()
                    .add(word::Column::Spelling.like(&pat))
                    .add(word::Column::Phonetic.like(&pat))
                    .add(word::Column::Example.like(&pat))
                    .add(Expr::cust("CAST(definitions AS TEXT)").like(&pat)),
            );
        }
        let column = match field {
            SortField::Spelling => word::Column::Spelling,
            SortField::CreatedAt => word::Column::CreatedAt,
            SortField::UpdatedAt => word::Column::UpdatedAt,
        };
        query = match dir {
            SortDir::Asc => query.order_by_asc(column),
            SortDir::Desc => query.order_by_desc(column),
        };
        let paginator = query.paginate(db, page_size);
        let models = paginator.fetch_page(page - 1).await?;
        let total = paginator.num_items().await?;
        Ok((models, total))
    }

    pub async fn find_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<word::Model>, sea_orm::DbErr> {
        word::Entity::find_by_id(id).one(db).await
    }

    pub async fn insert(
        db: &DatabaseConnection,
        model: word::ActiveModel,
    ) -> Result<word::Model, sea_orm::DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: word::ActiveModel,
    ) -> Result<word::Model, sea_orm::DbErr> {
        model.update(db).await
    }

    /// 删除单词；返回受影响行数（0 = 不存在）。
    pub async fn delete_by_id(db: &DatabaseConnection, id: i32) -> Result<u64, sea_orm::DbErr> {
        Ok(word::Entity::delete_by_id(id).exec(db).await?.rows_affected)
    }

    /// 批量插入（事务）：任一步失败整体回滚；返回插入行数。
    pub async fn insert_many(
        db: &DatabaseConnection,
        models: Vec<word::ActiveModel>,
    ) -> Result<u64, sea_orm::DbErr> {
        let n = models.len() as u64;
        let txn = db.begin().await?;
        word::Entity::insert_many(models).exec(&txn).await?;
        txn.commit().await?;
        Ok(n)
    }

    /// 批量删除（限定单词书归属）；返回受影响行数。
    pub async fn batch_delete(
        db: &DatabaseConnection,
        book_id: i32,
        ids: &[i32],
    ) -> Result<u64, sea_orm::DbErr> {
        Ok(word::Entity::delete_many()
            .filter(word::Column::WordbookId.eq(book_id))
            .filter(word::Column::Id.is_in(ids))
            .exec(db)
            .await?
            .rows_affected)
    }
}
