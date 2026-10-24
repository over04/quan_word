use serde::Deserialize;

/// 有道 suggest 接口的单条建议。
#[derive(Debug, Deserialize)]
pub struct SuggestItem {
    pub entry: String,
    pub explain: String,
}
