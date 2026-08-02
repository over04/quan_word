use serde::Deserialize;

/// 创建单词书请求体。
#[derive(Debug, Deserialize)]
pub struct CreateWordbookReq {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
}
