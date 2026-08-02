use entity::definition::Definition;
use serde::Deserialize;
use ts_rs::TS;

/// 创建单词请求体。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "CreateWordReq.ts")]
pub struct CreateWordReq {
    pub spelling: String,
    #[ts(optional)]
    pub phonetic: Option<String>,
    pub definitions: Vec<Definition>,
    #[ts(optional)]
    pub example: Option<String>,
}
