use crate::plugin_status::PluginStatus;
use crate::provider_id::ProviderId;

/// 单个插件文件的装载结果（成功或失败）。
#[derive(Debug, Clone)]
pub struct PluginLoadReport {
    pub file: String,
    pub id: Option<ProviderId>,
    pub status: PluginStatus,
    pub error: Option<String>,
}
