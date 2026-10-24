use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 插件能力：抓词 / 模糊建议。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "Capability.ts")]
pub enum Capability {
    Lookup,
    Suggest,
}
