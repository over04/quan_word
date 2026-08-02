use serde::Deserialize;
use ts_rs::TS;

/// 创建单词书请求体。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "CreateWordbookReq.ts")]
pub struct CreateWordbookReq {
    pub name: String,
    #[ts(optional)]
    pub description: Option<String>,
    #[ts(optional)]
    pub icon: Option<String>,
}
