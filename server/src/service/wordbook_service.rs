use std::collections::HashMap;

use chrono::Utc;
use entity::{word, wordbook};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use sea_orm::sea_query::{Expr, Func};

use crate::dto::req::create_wordbook_req::CreateWordbookReq;
use crate::dto::req::update_wordbook_req::UpdateWordbookReq;
use crate::dto::resp::wordbook_resp::WordbookResp;
use crate::error::ApiError;

/// 单词书业务逻辑：列表 / 创建 / 更新 / 删除。
pub struct WordbookService;

impl WordbookService {
    pub async fn list(db: &DatabaseConnection) -> Result<Vec<WordbookResp>, ApiError> {
        let books = wordbook::Entity::find()
            .order_by_asc(wordbook::Column::Id)
            .all(db)
            .await?;
        // 单次聚合查询所有单词书的单词数，避免每本书一次 count（N+1）
        let counts: HashMap<i32, i64> = word::Entity::find()
            .select_only()
            .column(word::Column::WordbookId)
            .column_as(Expr::expr(Func::count(Expr::col(word::Column::Id))), "cnt")
            .group_by(word::Column::WordbookId)
            .into_tuple()
            .all(db)
            .await?
            .into_iter()
            .collect();
        let resps = books
            .into_iter()
            .map(|b| {
                WordbookResp::new(
                    b.id,
                    b.name,
                    b.description,
                    b.icon,
                    counts.get(&b.id).copied().unwrap_or(0) as u64,
                )
            })
            .collect();
        Ok(resps)
    }

    pub async fn create(
        db: &DatabaseConnection,
        req: CreateWordbookReq,
    ) -> Result<WordbookResp, ApiError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ApiError::BadRequest("书名不能为空".into()));
        }
        let now = Utc::now();
        let model = wordbook::ActiveModel {
            name: Set(name),
            description: Set(req.description.unwrap_or_default()),
            icon: Set(req.icon.unwrap_or_else(|| "📖".into())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(WordbookResp::new(
            model.id,
            model.name,
            model.description,
            model.icon,
            0,
        ))
    }

    pub async fn update(
        db: &DatabaseConnection,
        id: i32,
        req: UpdateWordbookReq,
    ) -> Result<WordbookResp, ApiError> {
        let book = wordbook::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("单词书 {id} 不存在")))?;
        let mut model: wordbook::ActiveModel = book.into();
        if let Some(name) = req.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(ApiError::BadRequest("书名不能为空".into()));
            }
            model.name = Set(name);
        }
        if let Some(description) = req.description {
            model.description = Set(description);
        }
        if let Some(icon) = req.icon {
            model.icon = Set(icon);
        }
        model.updated_at = Set(Utc::now());
        let saved = model.update(db).await?;
        let word_count = Self::count_words(db, saved.id).await?;
        Ok(WordbookResp::new(
            saved.id,
            saved.name,
            saved.description,
            saved.icon,
            word_count,
        ))
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), ApiError> {
        let res = wordbook::Entity::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(ApiError::NotFound(format!("单词书 {id} 不存在")));
        }
        Ok(())
    }

    async fn count_words(db: &DatabaseConnection, book_id: i32) -> Result<u64, ApiError> {
        Ok(word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .count(db)
            .await?)
    }
}
