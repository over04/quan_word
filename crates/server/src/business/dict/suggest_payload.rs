use serde::Deserialize;

use super::suggest_item::SuggestItem;

/// 有道 suggest 接口的 data 字段。
#[derive(Debug, Deserialize)]
pub struct SuggestPayload {
    #[serde(default)]
    pub entries: Vec<SuggestItem>,
}
