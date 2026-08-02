use entity::definition::Definition;
use serde::Serialize;
use ts_rs::TS;

/// 单词响应体。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "Word.ts", rename = "Word")]
pub struct WordResp {
    pub id: i32,
    pub wordbook_id: i32,
    pub spelling: String,
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    pub example: Option<String>,
    /// RFC3339 字符串（ts-rs 无 chrono feature，用 String 而非 DateTimeUtc）
    pub created_at: String,
    pub updated_at: String,
}
