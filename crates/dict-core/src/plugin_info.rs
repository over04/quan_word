use crate::capability::Capability;
use crate::plugin_manifest::PluginManifest;
use crate::provider_id::ProviderId;

/// 已装载插件的公开信息（供 `/api/dict/providers` 与移动端 UI 使用）。
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub id: ProviderId,
    pub label: String,
    pub short_label: String,
    pub description: Option<String>,
    pub version: String,
    pub capabilities: Vec<Capability>,
    pub hosts: Vec<String>,
}

impl PluginInfo {
    pub fn from_manifest(manifest: &PluginManifest) -> Self {
        Self {
            id: manifest.id.clone(),
            label: manifest.label.clone(),
            short_label: manifest
                .short_label
                .clone()
                .unwrap_or_else(|| manifest.label.clone()),
            description: manifest.description.clone(),
            version: manifest.version.clone(),
            capabilities: manifest.capabilities.clone(),
            hosts: manifest.hosts.clone(),
        }
    }
}
