use serde::Deserialize;

use crate::model::definition::Definition;

/// 创建单词请求体。
#[derive(Debug, Deserialize)]
pub struct CreateWordReq {
    pub spelling: String,
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    pub example: Option<String>,
}
