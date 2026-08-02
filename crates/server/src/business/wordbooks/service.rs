use std::sync::Arc;

use chrono::Utc;
use entity::wordbook;
use sea_orm::Set;

use super::dto::create::CreateWordbookReq;
use super::dto::resp::WordbookResp;
use super::dto::update::UpdateWordbookReq;
use super::error::WordbookError;
use super::repo::WordbookRepo;
use crate::common::state::AppState;

/// 单词书业务逻辑：列表（内存缓存）/ 单书 / 创建 / 更新 / 删除。
pub struct WordbookService;

impl WordbookService {
    pub async fn list(state: &AppState) -> Result<Vec<WordbookResp>, WordbookError> {
        // 缓存命中直接返回（写操作同步失效，见 invalidate_wordbooks 触发点）
        if let Some(c) = state.wordbooks_cache.lock().as_ref() {
            return Ok((**c).clone());
        }
        let db = state.db.as_ref();
        let books = WordbookRepo::find_all(db).await?;
        let counts = WordbookRepo::count_words_by_book(db).await?;
        let resps: Vec<WordbookResp> = books
            .into_iter()
            .map(|b| WordbookResp {
                id: b.id,
                name: b.name,
                description: b.description,
                icon: b.icon,
                word_count: counts.get(&b.id).copied().unwrap_or(0) as u64,
            })
            .collect();
        *state.wordbooks_cache.lock() = Some(Arc::new(resps.clone()));
        Ok(resps)
    }

    /// 单书信息（含单词数）：单行查询 + count，不进列表缓存。
    pub async fn get(state: &AppState, id: i32) -> Result<WordbookResp, WordbookError> {
        let db = state.db.as_ref();
        let book = WordbookRepo::find_by_id(db, id)
            .await?
            .ok_or(WordbookError::WordbookNotFound { wordbook_id: id })?;
        let word_count = WordbookRepo::count_words(db, book.id).await?;
        Ok(WordbookResp {
            id: book.id,
            name: book.name,
            description: book.description,
            icon: book.icon,
            word_count,
        })
    }

    pub async fn create(
        state: &AppState,
        req: CreateWordbookReq,
    ) -> Result<WordbookResp, WordbookError> {
        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(WordbookError::EmptyName);
        }
        let now = Utc::now();
        let model = WordbookRepo::insert(
            state.db.as_ref(),
            wordbook::ActiveModel {
                name: Set(name),
                description: Set(req.description.unwrap_or_default()),
                icon: Set(req.icon.unwrap_or_else(|| "📖".into())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            },
        )
        .await?;
        state.invalidate_wordbooks();
        Ok(WordbookResp {
            id: model.id,
            name: model.name,
            description: model.description,
            icon: model.icon,
            word_count: 0,
        })
    }

    pub async fn update(
        state: &AppState,
        id: i32,
        req: UpdateWordbookReq,
    ) -> Result<WordbookResp, WordbookError> {
        let db = state.db.as_ref();
        let book = WordbookRepo::find_by_id(db, id)
            .await?
            .ok_or(WordbookError::WordbookNotFound { wordbook_id: id })?;
        let mut model: wordbook::ActiveModel = book.into();
        if let Some(name) = req.name {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Err(WordbookError::EmptyName);
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
        let saved = WordbookRepo::update(db, model).await?;
        let word_count = WordbookRepo::count_words(db, saved.id).await?;
        state.invalidate_wordbooks();
        Ok(WordbookResp {
            id: saved.id,
            name: saved.name,
            description: saved.description,
            icon: saved.icon,
            word_count,
        })
    }

    pub async fn delete(state: &AppState, id: i32) -> Result<(), WordbookError> {
        let rows = WordbookRepo::delete_by_id(state.db.as_ref(), id).await?;
        if rows == 0 {
            return Err(WordbookError::WordbookNotFound { wordbook_id: id });
        }
        state.invalidate_wordbooks();
        Ok(())
    }
}
