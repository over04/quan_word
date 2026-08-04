use sea_orm::DbErr;

use crate::common::error::ApiError;

/// 单词领域错误。
#[derive(Debug, thiserror::Error)]
pub enum WordError {
    #[error("单词书 {wordbook_id} 不存在")]
    WordbookNotFound { wordbook_id: i32 },
    #[error("单词 {word_id} 不存在")]
    WordNotFound { word_id: i32 },
    #[error("单词 {word_id} 不属于单词书 {wordbook_id}")]
    WordNotInWordbook { word_id: i32, wordbook_id: i32 },
    #[error("单词不能为空")]
    EmptySpelling,
    #[error("至少需要一个释义")]
    EmptyDefinitions,
    #[error("释义内容不能为空")]
    EmptyMeaning,
    #[error("词性不合法: {pos}，可选：n. / c / C / u / U / cu / CU / v. / vt. / vi. / adj. / adv. / prep. / conj. / pron. / num. / art. / interj. / aux. / abbr. / phr. 或留空")]
    InvalidPos { pos: String },
    #[error("不支持的排序: {order}")]
    InvalidOrder { order: String },
    #[error("order=random 需要 seed 参数")]
    RandomWithoutSeed,
    #[error("不支持的排序字段: {field}")]
    InvalidSortField { field: String },
    #[error("order 必须为 asc 或 desc: {dir}")]
    InvalidSortDir { dir: String },
    #[error("不支持的标签匹配模式: {mode}，可选：and / or")]
    InvalidTagMatch { mode: String },
    #[error("标签参数不合法: {tag}")]
    InvalidTagIds { tag: String },
    #[error("标签 {tag_id} 不属于单词书 {wordbook_id}")]
    TagNotInWordbook { tag_id: i32, wordbook_id: i32 },
    #[error("请选择要添加的标签")]
    EmptyTagSelection,
    #[error("释义数据格式错误: {0}")]
    DefinitionsJson(#[from] serde_json::Error),
    #[error("未选择要删除的单词")]
    EmptySelection,
    #[error("不支持的文件格式: {ext}，仅支持 csv / xlsx / xls / ods")]
    UnsupportedFormat { ext: String },
    #[error("导入会话无效或已过期，请重新上传文件预览")]
    ImportSessionInvalid,
    #[error("导入失败：共 {count} 行有误\n{details}")]
    ImportFailed { count: usize, details: String },
    #[error("导入行数超过上限（{limit} 行）")]
    TooManyRows { limit: usize },
    #[error("模板生成失败: {0}")]
    Template(String),
    #[error(transparent)]
    Db(#[from] DbErr),
}

impl From<WordError> for ApiError {
    fn from(e: WordError) -> Self {
        match e {
            WordError::WordbookNotFound { .. }
            | WordError::WordNotFound { .. }
            | WordError::WordNotInWordbook { .. } => ApiError::NotFound(e.to_string()),
            WordError::Db(e) => ApiError::Db(e),
            _ => ApiError::BadRequest(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WordError;
    use crate::common::error::ApiError;

    #[test]
    fn not_found_maps_to_404_with_message() {
        match ApiError::from(WordError::WordbookNotFound { wordbook_id: 7 }) {
            ApiError::NotFound(m) => assert!(m.contains("7")),
            _ => panic!("应为 NotFound"),
        }
        match ApiError::from(WordError::WordNotFound { word_id: 3 }) {
            ApiError::NotFound(m) => assert!(m.contains("3")),
            _ => panic!("应为 NotFound"),
        }
    }

    #[test]
    fn validation_maps_to_400() {
        assert!(matches!(
            ApiError::from(WordError::EmptySpelling),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::InvalidOrder { order: "x".into() }),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::ImportFailed {
                count: 2,
                details: "第 2 行：释义不能为空".into(),
            }),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::EmptySelection),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::TagNotInWordbook {
                tag_id: 1,
                wordbook_id: 2,
            }),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::EmptyTagSelection),
            ApiError::BadRequest(_)
        ));
        assert!(matches!(
            ApiError::from(WordError::InvalidTagIds { tag: "abc".into() }),
            ApiError::BadRequest(_)
        ));
    }
}
