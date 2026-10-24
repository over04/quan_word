use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 插件允许的请求方式：GET 或表单 POST。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "RequestMethod.ts")]
pub enum RequestMethod {
    Get,
    PostForm,
}
