use serde::Serialize;

use crate::model::definition::Definition;

/// 单词响应体。
#[derive(Debug, Clone, Serialize)]
pub struct WordResp {
    pub id: i32,
    pub wordbook_id: i32,
    pub spelling: String,
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    pub example: Option<String>,
}
