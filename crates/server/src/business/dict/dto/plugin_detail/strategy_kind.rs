use serde::Serialize;
use ts_rs::TS;

/// 释义生成策略类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "PluginStrategyKind.ts")]
pub enum PluginStrategyKind {
    PosText,
    PlainText,
    Items,
    Groups,
}
