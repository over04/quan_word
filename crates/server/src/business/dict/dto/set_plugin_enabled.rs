use serde::Deserialize;
use ts_rs::TS;

/// 运行时启停单个词典源。
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "SetPluginEnabledReq.ts")]
pub struct SetPluginEnabledReq {
    pub enabled: bool,
}
