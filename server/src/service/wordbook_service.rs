use std::collections::HashMap;
use std::sync::Arc;

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
use crate::state::AppState;

/// 单词书业务逻辑：列表（内存缓存）/ 单书 / 创建 / 更新 / 删除。
pub struct WordbookService;

impl WordbookService {
    pub async fn list(state: &AppState) -> Result<Vec<WordbookResp>, ApiError> {
        // 缓存命中直接返回（写操作同步失效，见 invalidate_wordbooks 触发点）
        if let Some(c) = state.wordbooks_cache.lock().as_ref() {
            return Ok((**c).clone());
        }
        let db = state.db.as_ref();
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
        let resps: Vec<WordbookResp> = books
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
        *state.wordbooks_cache.lock() = Some(Arc::new(resps.clone()));
        Ok(resps)
    }

    /// 单书信息（含单词数）：单行查询 + count，不进列表缓存。
    pub async fn get(state: &AppState, id: i32) -> Result<WordbookResp, ApiError> {
        let db = state.db.as_ref();
        let book = wordbook::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("单词书 {id} 不存在")))?;
        let word_count = Self::count_words(db, book.id).await?;
        Ok(WordbookResp::new(
            book.id,
            book.name,
            book.description,
            book.icon,
            word_count,
        ))
    }

    pub async fn create(
        state: &AppState,
        req: CreateWordbookReq,
    ) -> Result<WordbookResp, ApiError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ApiError::BadRequest("书名不能为空".into()));
        }
        let now = Utc::now();
        let db = state.db.as_ref();
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
        invalidate_wordbooks(state);
        Ok(WordbookResp::new(
            model.id,
            model.name,
            model.description,
            model.icon,
            0,
        ))
    }

    pub async fn update(
        state: &AppState,
        id: i32,
        req: UpdateWordbookReq,
    ) -> Result<WordbookResp, ApiError> {
        let db = state.db.as_ref();
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
        invalidate_wordbooks(state);
        Ok(WordbookResp::new(
            saved.id,
            saved.name,
            saved.description,
            saved.icon,
            word_count,
        ))
    }

    pub async fn delete(state: &AppState, id: i32) -> Result<(), ApiError> {
        let db = state.db.as_ref();
        let res = wordbook::Entity::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(ApiError::NotFound(format!("单词书 {id} 不存在")));
        }
        invalidate_wordbooks(state);
        Ok(())
    }

    async fn count_words(db: &DatabaseConnection, book_id: i32) -> Result<u64, ApiError> {
        Ok(word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .count(db)
            .await?)
    }
}

/// 失效单词书列表缓存：单词书增删改、单词增删（影响 word_count）后调用。
pub(crate) fn invalidate_wordbooks(state: &AppState) {
    *state.wordbooks_cache.lock() = None;
}
