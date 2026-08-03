use std::collections::{HashMap, HashSet};

use chrono::Utc;
use entity::{tag, word, word_tag, wordbook};
use sea_orm::sea_query::{Condition, Expr, ExprTrait, Query};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Select, Set, TransactionTrait,
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

    /// 浏览模式分页（id / 字母序，SQL 层排序）；`tag_ids` 非空时按标签交集筛选。
    pub async fn browse_page(
        db: &DatabaseConnection,
        book_id: i32,
        column: word::Column,
        dir: SortDir,
        page: u64,
        page_size: u64,
        tag_ids: &[i32],
    ) -> Result<(Vec<word::Model>, u64), sea_orm::DbErr> {
        let mut q = Self::with_tag_filter(
            word::Entity::find().filter(word::Column::WordbookId.eq(book_id)),
            tag_ids,
        );
        q = match dir {
            SortDir::Asc => q.order_by_asc(column),
            SortDir::Desc => q.order_by_desc(column),
        };
        let paginator = q.paginate(db, page_size);
        let models = paginator.fetch_page(page - 1).await?;
        let total = paginator.num_items().await?;
        Ok((models, total))
    }

    /// 某本书（可带标签筛选）的全部单词 id（供 seeded 洗牌）。
    pub async fn find_all_ids(
        db: &DatabaseConnection,
        book_id: i32,
        tag_ids: &[i32],
    ) -> Result<Vec<i32>, sea_orm::DbErr> {
        Self::with_tag_filter(
            word::Entity::find()
                .select_only()
                .column(word::Column::Id)
                .filter(word::Column::WordbookId.eq(book_id)),
            tag_ids,
        )
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

    /// 列表模式查询：书内搜索（拼写/释义模糊匹配）+ 排序 + 标签交集筛选 + 分页。
    #[allow(clippy::too_many_arguments)]
    pub async fn search_page(
        db: &DatabaseConnection,
        book_id: i32,
        q: Option<&str>,
        field: SortField,
        dir: SortDir,
        page: u64,
        page_size: u64,
        tag_ids: &[i32],
    ) -> Result<(Vec<word::Model>, u64), sea_orm::DbErr> {
        let mut query = Self::with_tag_filter(
            word::Entity::find().filter(word::Column::WordbookId.eq(book_id)),
            tag_ids,
        );
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

    /// 校验标签集合归属：返回该书内实际存在的标签 id（去重、升序）。
    pub async fn find_tag_ids_by_book(
        db: &DatabaseConnection,
        book_id: i32,
        ids: &[i32],
    ) -> Result<Vec<i32>, sea_orm::DbErr> {
        let mut found: Vec<i32> = tag::Entity::find()
            .select_only()
            .column(tag::Column::Id)
            .filter(tag::Column::WordbookId.eq(book_id))
            .filter(tag::Column::Id.is_in(ids.iter().copied()))
            .into_tuple()
            .all(db)
            .await?;
        found.sort_unstable();
        Ok(found)
    }

    /// 一批单词各自的标签 id（word_id → 排序去重的 tag_id 列表）。
    pub async fn find_tag_ids_by_word_ids(
        db: &DatabaseConnection,
        ids: &[i32],
    ) -> Result<HashMap<i32, Vec<i32>>, sea_orm::DbErr> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = word_tag::Entity::find()
            .filter(word_tag::Column::WordId.is_in(ids.iter().copied()))
            .all(db)
            .await?;
        let mut map: HashMap<i32, Vec<i32>> = HashMap::new();
        for r in rows {
            map.entry(r.word_id).or_default().push(r.tag_id);
        }
        for v in map.values_mut() {
            v.sort_unstable();
            v.dedup();
        }
        Ok(map)
    }

    /// 插入单词并在同一事务内写入标签关联；返回 (模型, 去重后的 tag_ids)。
    pub async fn insert_with_tags(
        db: &DatabaseConnection,
        model: word::ActiveModel,
        tag_ids: &[i32],
    ) -> Result<(word::Model, Vec<i32>), sea_orm::DbErr> {
        let txn = db.begin().await?;
        let saved = model.insert(&txn).await?;
        Self::insert_tag_links(&txn, saved.id, tag_ids).await?;
        txn.commit().await?;
        Ok((saved, tag_ids.to_vec()))
    }

    /// 更新单词并在同一事务内重建标签关联（先删后插）；返回 (模型, 去重后的 tag_ids)。
    pub async fn update_with_tags(
        db: &DatabaseConnection,
        model: word::ActiveModel,
        tag_ids: &[i32],
    ) -> Result<(word::Model, Vec<i32>), sea_orm::DbErr> {
        let txn = db.begin().await?;
        let saved = model.update(&txn).await?;
        word_tag::Entity::delete_many()
            .filter(word_tag::Column::WordId.eq(saved.id))
            .exec(&txn)
            .await?;
        Self::insert_tag_links(&txn, saved.id, tag_ids).await?;
        txn.commit().await?;
        Ok((saved, tag_ids.to_vec()))
    }

    /// 批量打标签（只添加，跳过已存在的关联）；返回实际插入行数。
    pub async fn batch_tag(
        db: &DatabaseConnection,
        book_id: i32,
        word_ids: &[i32],
        tag_ids: &[i32],
    ) -> Result<u64, sea_orm::DbErr> {
        if word_ids.is_empty() || tag_ids.is_empty() {
            return Ok(0);
        }
        let valid_ids: Vec<i32> = word::Entity::find()
            .select_only()
            .column(word::Column::Id)
            .filter(word::Column::WordbookId.eq(book_id))
            .filter(word::Column::Id.is_in(word_ids.iter().copied()))
            .into_tuple()
            .all(db)
            .await?;
        if valid_ids.is_empty() {
            return Ok(0);
        }
        let existing: HashSet<(i32, i32)> = word_tag::Entity::find()
            .filter(word_tag::Column::WordId.is_in(valid_ids.iter().copied()))
            .filter(word_tag::Column::TagId.is_in(tag_ids.iter().copied()))
            .all(db)
            .await?
            .into_iter()
            .map(|r| (r.word_id, r.tag_id))
            .collect();
        let mut models = Vec::new();
        for wid in &valid_ids {
            for tid in tag_ids {
                if !existing.contains(&(*wid, *tid)) {
                    models.push(word_tag::ActiveModel {
                        word_id: Set(*wid),
                        tag_id: Set(*tid),
                    });
                }
            }
        }
        if models.is_empty() {
            return Ok(0);
        }
        let n = models.len() as u64;
        let txn = db.begin().await?;
        word_tag::Entity::insert_many(models).exec(&txn).await?;
        txn.commit().await?;
        Ok(n)
    }

    pub async fn delete_by_id(db: &DatabaseConnection, id: i32) -> Result<u64, sea_orm::DbErr> {
        Ok(word::Entity::delete_by_id(id).exec(db).await?.rows_affected)
    }

    /// 该书全部标签名 → id 映射。
    pub async fn find_tag_map(
        db: &DatabaseConnection,
        book_id: i32,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        Ok(tag::Entity::find()
            .filter(tag::Column::WordbookId.eq(book_id))
            .all(db)
            .await?
            .into_iter()
            .map(|t| (t.name, t.id))
            .collect())
    }

    /// 该书全部单词拼写（trim + 小写）→ id 映射（导入重复判定用）。
    /// 同拼写多词时保留最早（id 最小）的，保证更新目标确定性。
    pub async fn find_spellings(
        db: &DatabaseConnection,
        book_id: i32,
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        let mut map = HashMap::new();
        for w in word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .order_by_asc(word::Column::Id)
            .all(db)
            .await?
        {
            map.entry(w.spelling.trim().to_lowercase()).or_insert(w.id);
        }
        Ok(map)
    }

    /// 批量创建标签（names 已去重校验）；返回 name → id（含全部传入项）。
    pub async fn insert_tags(
        db: &DatabaseConnection,
        book_id: i32,
        names: &[String],
    ) -> Result<HashMap<String, i32>, sea_orm::DbErr> {
        let mut map = HashMap::with_capacity(names.len());
        if names.is_empty() {
            return Ok(map);
        }
        let now = Utc::now();
        let txn = db.begin().await?;
        for name in names {
            let model = tag::ActiveModel {
                wordbook_id: Set(book_id),
                name: Set(name.clone()),
                created_at: Set(now),
                ..Default::default()
            };
            let saved = model.insert(&txn).await?;
            map.insert(saved.name, saved.id);
        }
        txn.commit().await?;
        Ok(map)
    }

    /// 事务执行导入落库：插入新词 + 更新重复词 + 写标签关联；任一步失败整体回滚。
    /// inserts: (word model, tag_ids)；updates: (word model, 合并后的 tag_ids 全集)。
    pub async fn import_inserts(
        db: &DatabaseConnection,
        inserts: &[(word::ActiveModel, Vec<i32>)],
        updates: &[(word::ActiveModel, Vec<i32>)],
    ) -> Result<(), sea_orm::DbErr> {
        if inserts.is_empty() && updates.is_empty() {
            return Ok(());
        }
        let txn = db.begin().await?;
        for (model, tag_ids) in inserts {
            let saved = model.clone().insert(&txn).await?;
            Self::insert_tag_links(&txn, saved.id, tag_ids).await?;
        }
        for (model, tag_ids) in updates {
            let saved = model.clone().update(&txn).await?;
            word_tag::Entity::delete_many()
                .filter(word_tag::Column::WordId.eq(saved.id))
                .exec(&txn)
                .await?;
            Self::insert_tag_links(&txn, saved.id, tag_ids).await?;
        }
        txn.commit().await?;
        Ok(())
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

    /// 标签交集筛选：word 必须同时拥有全部 tag_ids。
    /// 子查询：`word_id IN (SELECT word_id FROM word_tag WHERE tag_id IN (...) GROUP BY word_id HAVING COUNT(DISTINCT tag_id) = N)`。
    fn with_tag_filter(mut query: Select<word::Entity>, tag_ids: &[i32]) -> Select<word::Entity> {
        if tag_ids.is_empty() {
            return query;
        }
        let sub = Query::select()
            .column((word_tag::Entity, word_tag::Column::WordId))
            .from(word_tag::Entity)
            .cond_where(word_tag::Column::TagId.is_in(tag_ids.iter().copied()))
            .group_by_col((word_tag::Entity, word_tag::Column::WordId))
            .cond_having(
                Expr::col((word_tag::Entity, word_tag::Column::TagId))
                    .count_distinct()
                    .eq(tag_ids.len() as u64),
            )
            .to_owned();
        query = query.filter(word::Column::Id.in_subquery(sub));
        query
    }

    /// 事务内写入一个单词的标签关联（幂等：先删后插由调用方决定；此处仅插入）。
    async fn insert_tag_links(
        txn: &sea_orm::DatabaseTransaction,
        word_id: i32,
        tag_ids: &[i32],
    ) -> Result<(), sea_orm::DbErr> {
        if tag_ids.is_empty() {
            return Ok(());
        }
        let models: Vec<word_tag::ActiveModel> = tag_ids
            .iter()
            .map(|tid| word_tag::ActiveModel {
                word_id: Set(word_id),
                tag_id: Set(*tid),
            })
            .collect();
        word_tag::Entity::insert_many(models).exec(txn).await?;
        Ok(())
    }
}
