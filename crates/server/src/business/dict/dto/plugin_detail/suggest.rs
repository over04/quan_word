use serde::Serialize;
use ts_rs::TS;

/// 模糊建议配方的可视化摘要。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginSuggestResp.ts")]
pub struct DictPluginSuggestResp {
    pub fields: Vec<String>,
    pub take: Option<usize>,
    pub dedup: Option<String>,
}
