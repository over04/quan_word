use serde::Serialize;
use ts_rs::TS;

use super::suggestion::DictSuggestion;

/// 模糊查词建议响应。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictSuggestResp.ts")]
pub struct DictSuggestResp {
    pub word: String,
    pub suggestions: Vec<DictSuggestion>,
}
