use serde::Serialize;
use ts_rs::TS;

use super::strategy_kind::PluginStrategyKind;

/// 单条释义策略的可视化摘要。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginStrategyResp.ts")]
pub struct DictPluginStrategyResp {
    pub kind: PluginStrategyKind,
    pub has_pos: bool,
    pub has_condition: bool,
    pub transforms: usize,
}
