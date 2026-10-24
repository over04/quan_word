use serde::Serialize;
use ts_rs::TS;

use super::field::DictPluginFieldResp;
use super::strategy::DictPluginStrategyResp;

/// 抓词配方的可视化摘要。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginLookupResp.ts")]
pub struct DictPluginLookupResp {
    pub fields: Vec<DictPluginFieldResp>,
    pub strategies: Vec<DictPluginStrategyResp>,
    pub vars: Vec<String>,
    pub empty_is_parse: bool,
}
