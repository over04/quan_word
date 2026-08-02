use entity::definition::Definition;
use serde::Deserialize;

/// 更新单词请求体（全量更新，与创建同字段）。
#[derive(Debug, Deserialize)]
pub struct UpdateWordReq {
    pub spelling: String,
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    pub example: Option<String>,
}
