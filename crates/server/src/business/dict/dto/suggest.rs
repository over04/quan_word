use serde::Deserialize;

/// 模糊查词建议查询参数。
#[derive(Debug, Deserialize)]
pub struct SuggestQuery {
    pub word: Option<String>,
}
