use sea_orm::DbErr;

use crate::common::error::ApiError;

/// 单词书领域错误。
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum WordbookError {
    #[error("单词书 {wordbook_id} 不存在")]
    WordbookNotFound { wordbook_id: i32 },
    #[error("书名不能为空")]
    EmptyName,
    #[error(transparent)]
    Db(#[from] DbErr),
}

impl From<WordbookError> for ApiError {
    fn from(e: WordbookError) -> Self {
        match e {
            WordbookError::WordbookNotFound { .. } => ApiError::NotFound(e.to_string()),
            WordbookError::EmptyName => ApiError::BadRequest(e.to_string()),
            WordbookError::Db(e) => ApiError::Db(e),
        }
    }
}
