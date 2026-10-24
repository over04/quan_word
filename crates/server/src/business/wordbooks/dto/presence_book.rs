use serde::Serialize;
use ts_rs::TS;

/// 某个单词所在的书与词条 id。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "WordPresenceBook.ts")]
pub struct WordPresenceBook {
    pub book_id: i32,
    pub book_name: String,
    pub word_id: i32,
}
