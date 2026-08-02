use serde::Serialize;
use ts_rs::TS;

/// 标签响应体（含使用该标签的单词数）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "Tag.ts", rename = "Tag")]
pub struct TagResp {
    pub id: i32,
    pub name: String,
    #[ts(type = "number")]
    pub word_count: u64,
}
