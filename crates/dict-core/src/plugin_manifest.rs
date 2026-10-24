use serde::Deserialize;

use crate::capability::Capability;
use crate::plugin_engine::PluginEngine;
use crate::plugin_limits::PluginLimits;
use crate::provider_id::ProviderId;

/// 插件清单：身份、展示名、能力、允许访问的主机与上限。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub api_version: u32,
    pub id: ProviderId,
    pub label: String,
    #[serde(default)]
    pub short_label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub version: String,
    pub engine: PluginEngine,
    #[serde(default = "default_capabilities")]
    pub capabilities: Vec<Capability>,
    pub hosts: Vec<String>,
    #[serde(default)]
    pub limits: PluginLimits,
}

fn default_capabilities() -> Vec<Capability> {
    vec![Capability::Lookup]
}
