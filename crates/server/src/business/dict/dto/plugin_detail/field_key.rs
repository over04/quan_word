use serde::Serialize;
use ts_rs::TS;

/// 插件注册的抓词字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "PluginFieldKey.ts")]
pub enum PluginFieldKey {
    Word,
    Phonetic,
    Audio,
    Source,
    Definitions,
    Examples,
}
