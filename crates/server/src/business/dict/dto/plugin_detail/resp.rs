use dict_core::capability::Capability;
use dict_core::provider_id::ProviderId;
use serde::Serialize;
use ts_rs::TS;

use super::limits::DictPluginLimitsResp;
use super::lookup::DictPluginLookupResp;
use super::request::DictPluginRequestResp;
use super::suggest::DictPluginSuggestResp;

/// 单个插件的详情蓝图：能力、注册内容、请求与上限。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginDetailResp.ts")]
pub struct DictPluginDetailResp {
    pub id: ProviderId,
    pub label: String,
    pub short_label: String,
    pub description: Option<String>,
    pub version: String,
    pub enabled: bool,
    pub capabilities: Vec<Capability>,
    pub hosts: Vec<String>,
    pub lookup_request: Option<DictPluginRequestResp>,
    pub lookup: Option<DictPluginLookupResp>,
    pub suggest_request: Option<DictPluginRequestResp>,
    pub suggest: Option<DictPluginSuggestResp>,
    pub limits: DictPluginLimitsResp,
}
