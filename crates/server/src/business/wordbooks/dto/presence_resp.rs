use serde::Serialize;
use ts_rs::TS;

use super::presence_book::WordPresenceBook;

/// 某拼写在全部单词书中的存在情况。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "WordPresenceResp.ts")]
pub struct WordPresenceResp {
    pub spelling: String,
    pub books: Vec<WordPresenceBook>,
}
