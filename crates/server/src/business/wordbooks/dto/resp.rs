use serde::Serialize;
use ts_rs::TS;

/// 单词书响应体（含单词数）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "Wordbook.ts", rename = "Wordbook")]
pub struct WordbookResp {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[ts(type = "number")]
    pub word_count: u64,
}
