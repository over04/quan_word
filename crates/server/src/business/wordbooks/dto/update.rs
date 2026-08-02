use serde::Deserialize;

/// 更新单词书请求体：字段为 None 时保留原值。
#[derive(Debug, Deserialize)]
pub struct UpdateWordbookReq {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}
