use serde::Deserialize;
use ts_rs::TS;

/// 创建标签请求体。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "CreateTagReq.ts")]
pub struct CreateTagReq {
    pub name: String,
}
