use chrono::Utc;
use entity::tag;
use entity::wordbook;
use sea_orm::{DatabaseConnection, EntityTrait, Set};

use super::dto::create::CreateTagReq;
use super::dto::resp::TagResp;
use super::dto::update::UpdateTagReq;
use super::error::TagError;
use super::repo::TagRepo;
use crate::common::state::AppState;

/// 标签名长度上限（字符数）。
const TAG_NAME_MAX: usize = 20;

/// 标签业务逻辑：列表（含单词数）/ 创建 / 重命名 / 删除。
pub struct TagService;

impl TagService {
    pub async fn list(state: &AppState, book_id: i32) -> Result<Vec<TagResp>, TagError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        Ok(TagRepo::find_all_with_counts(db, book_id).await?)
    }

    pub async fn create(
        state: &AppState,
        book_id: i32,
        req: CreateTagReq,
    ) -> Result<TagResp, TagError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let name = Self::validate_name(&req.name)?;
        if TagRepo::find_dup(db, book_id, &name, None).await?.is_some() {
            return Err(TagError::TagDuplicate { name });
        }
        let model = TagRepo::insert(
            db,
            tag::ActiveModel {
                wordbook_id: Set(book_id),
                name: Set(name),
                created_at: Set(Utc::now()),
                ..Default::default()
            },
        )
        .await?;
        Ok(TagResp {
            id: model.id,
            name: model.name,
            word_count: 0,
        })
    }

    pub async fn update(
        state: &AppState,
        book_id: i32,
        id: i32,
        req: UpdateTagReq,
    ) -> Result<TagResp, TagError> {
        let db = state.db.as_ref();
        let t = TagRepo::find_by_id(db, id)
            .await?
            .ok_or(TagError::TagNotFound { tag_id: id })?;
        if t.wordbook_id != book_id {
            return Err(TagError::TagNotInWordbook {
                tag_id: id,
                wordbook_id: book_id,
            });
        }
        let name = Self::validate_name(&req.name)?;
        if TagRepo::find_dup(db, book_id, &name, Some(id))
            .await?
            .is_some()
        {
            return Err(TagError::TagDuplicate { name });
        }
        let mut model: tag::ActiveModel = t.into();
        model.name = Set(name);
        let saved = TagRepo::update(db, model).await?;
        Ok(TagResp {
            id: saved.id,
            name: saved.name,
            word_count: TagRepo::count_word(db, saved.id).await?,
        })
    }

    pub async fn delete(state: &AppState, book_id: i32, id: i32) -> Result<(), TagError> {
        let db = state.db.as_ref();
        let t = TagRepo::find_by_id(db, id)
            .await?
            .ok_or(TagError::TagNotFound { tag_id: id })?;
        if t.wordbook_id != book_id {
            return Err(TagError::TagNotInWordbook {
                tag_id: id,
                wordbook_id: book_id,
            });
        }
        let rows = TagRepo::delete_by_id(db, id).await?;
        if rows == 0 {
            return Err(TagError::TagNotFound { tag_id: id });
        }
        Ok(())
    }

    async fn ensure_book_exists(db: &DatabaseConnection, book_id: i32) -> Result<(), TagError> {
        if wordbook::Entity::find_by_id(book_id)
            .one(db)
            .await?
            .is_none()
        {
            return Err(TagError::WordbookNotFound {
                wordbook_id: book_id,
            });
        }
        Ok(())
    }

    /// 标签名校验：trim 后非空、不超过上限；返回规范化后的名称。
    pub(crate) fn validate_name(name: &str) -> Result<String, TagError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TagError::EmptyTagName);
        }
        if trimmed.chars().count() > TAG_NAME_MAX {
            return Err(TagError::TagNameTooLong { max: TAG_NAME_MAX });
        }
        Ok(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TagService;
    use crate::business::wordbooks::tags::error::TagError;

    #[test]
    fn validate_rejects_empty_name() {
        assert!(matches!(
            TagService::validate_name(""),
            Err(TagError::EmptyTagName)
        ));
        assert!(matches!(
            TagService::validate_name("   "),
            Err(TagError::EmptyTagName)
        ));
    }

    #[test]
    fn validate_rejects_too_long_name() {
        assert!(matches!(
            TagService::validate_name(&"长".repeat(21)),
            Err(TagError::TagNameTooLong { max: 20 })
        ));
    }

    #[test]
    fn validate_accepts_at_most_20_chars() {
        assert_eq!(
            TagService::validate_name(&"短".repeat(20)).unwrap(),
            "短".repeat(20)
        );
    }

    #[test]
    fn validate_trims_name() {
        assert_eq!(TagService::validate_name("  高频  ").unwrap(), "高频");
    }
}
