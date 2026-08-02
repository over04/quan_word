use serde::Serialize;

/// 单词书响应体（含单词数）。
#[derive(Debug, Clone, Serialize)]
pub struct WordbookResp {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub word_count: u64,
}

impl WordbookResp {
    pub fn new(id: i32, name: String, description: String, icon: String, word_count: u64) -> Self {
        Self {
            id,
            name,
            description,
            icon,
            word_count,
        }
    }
}
