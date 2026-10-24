use dict_core::request_method::RequestMethod;
use serde::Serialize;
use ts_rs::TS;

/// 一次上游请求的可视化摘要（不暴露请求头/表单值）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictPluginRequestResp.ts")]
pub struct DictPluginRequestResp {
    pub method: RequestMethod,
    pub path: String,
    pub headers: Vec<String>,
    pub form: Vec<String>,
}
