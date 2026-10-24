use serde::Serialize;
use ts_rs::TS;

/// 插件声明的上限与引擎硬上限（用于环形刻度比例）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginLimitsResp.ts")]
pub struct DictPluginLimitsResp {
    pub max_definitions: usize,
    pub max_examples: usize,
    pub max_body_kib: usize,
    pub ceiling_definitions: usize,
    pub ceiling_examples: usize,
    pub ceiling_body_kib: usize,
}
