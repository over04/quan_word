use sea_orm::DbErr;

use crate::common::error::ApiError;

/// 标签领域错误。
#[derive(Debug, thiserror::Error)]
pub enum TagError {
    #[error("单词书 {wordbook_id} 不存在")]
    WordbookNotFound { wordbook_id: i32 },
    #[error("标签 {tag_id} 不存在")]
    TagNotFound { tag_id: i32 },
    #[error("标签 {tag_id} 不属于单词书 {wordbook_id}")]
    TagNotInWordbook { tag_id: i32, wordbook_id: i32 },
    #[error("标签名不能为空")]
    EmptyTagName,
    #[error("标签名不能超过 {max} 个字符")]
    TagNameTooLong { max: usize },
    #[error("标签「{name}」已存在")]
    TagDuplicate { name: String },
    #[error(transparent)]
    Db(#[from] DbErr),
}

impl From<TagError> for ApiError {
    fn from(e: TagError) -> Self {
        match e {
            TagError::WordbookNotFound { .. }
            | TagError::TagNotFound { .. }
            | TagError::TagNotInWordbook { .. } => ApiError::NotFound(e.to_string()),
            TagError::Db(e) => ApiError::Db(e),
            _ => ApiError::BadRequest(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TagError;
    use crate::common::error::ApiError;

    #[test]
    fn not_found_maps_to_404_with_message() {
        match ApiError::from(TagError::WordbookNotFound { wordbook_id: 7 }) {
            ApiError::NotFound(m) => assert!(m.contains("7")),
            _ => panic!("应为 NotFound"),
        }
        match ApiError::from(TagError::TagNotFound { tag_id: 3 }) {
            ApiError::NotFound(m) => assert!(m.contains("3")),
            _ => panic!("应为 NotFound"),
        }
        match ApiError::from(TagError::TagNotInWordbook {
            tag_id: 3,
            wordbook_id: 7,
        }) {
            ApiError::NotFound(m) => assert!(m.contains("3") && m.contains("7")),
            _ => panic!("应为 NotFound"),
        }
    }

    #[test]
    fn validation_maps_to_400() {
        assert!(matches!(
            ApiError::from(TagError::EmptyTagName),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(TagError::TagNameTooLong { max: 20 }),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(TagError::TagDuplicate { name: "x".into() }),
            ApiError::BadRequest(_)
        ));
    }
}
