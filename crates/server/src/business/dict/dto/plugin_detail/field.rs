use serde::Serialize;
use ts_rs::TS;

use super::field_key::PluginFieldKey;

/// 一个已注册字段：数量与子字段名（如例句的 text/translation）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginFieldResp.ts")]
pub struct DictPluginFieldResp {
    pub key: PluginFieldKey,
    pub count: usize,
    pub names: Vec<String>,
}
