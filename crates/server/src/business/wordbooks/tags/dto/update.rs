use serde::Deserialize;
use ts_rs::TS;

/// 更新标签请求体。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "UpdateTagReq.ts")]
pub struct UpdateTagReq {
    pub name: String,
}
