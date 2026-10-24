use serde::Serialize;
use ts_rs::TS;

/// 模糊查词建议：单词 + 简短释义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "DictSuggestion.ts")]
pub struct DictSuggestion {
    pub word: String,
    pub summary: String,
}
