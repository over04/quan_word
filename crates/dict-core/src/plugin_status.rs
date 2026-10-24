use serde::{Deserialize, Serialize};

/// 插件装载状态（仅服务端内部使用，不导出前端类型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Loaded,
    Rejected,
}
