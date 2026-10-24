use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::capability::Capability;
use crate::engine_limits::EngineLimits;
use crate::entry::DictEntry;
use crate::plugin_info::PluginInfo;
use crate::plugin_load_report::PluginLoadReport;
use crate::plugin_registry::PluginRegistry;
use crate::plugin_source::PluginSource;
use crate::provider_error::DictProviderError;
use crate::provider_id::ProviderId;
use crate::registry_entry::RegistryEntry;
use crate::reload_report::ReloadReport;
use crate::suggestion::DictSuggestion;
use crate::transport::SearchTransport;
use crate::transport_error::TransportError;

/// 平台无关的搜索门面：宿主注入传输层与插件字节，内核负责装载、查找与解析。
pub struct SearchEngine {
    transport: Arc<dyn SearchTransport>,
    limits: EngineLimits,
    registry: ArcSwap<PluginRegistry>,
}

impl SearchEngine {
    pub fn new(transport: Arc<dyn SearchTransport>, limits: EngineLimits) -> Self {
        Self {
            transport,
            limits,
            registry: ArcSwap::from_pointee(PluginRegistry::default()),
        }
    }

    pub fn limits(&self) -> &EngineLimits {
        &self.limits
    }

    /// 原子替换注册表；进行中的请求继续持有旧注册表的 `Arc`。
    pub fn load(&self, sources: Vec<PluginSource>) -> ReloadReport {
        let registry = PluginRegistry::build(sources, &self.limits);
        let reports = registry.reports().to_vec();
        let loaded = reports
            .iter()
            .filter(|report| report.status == crate::plugin_status::PluginStatus::Loaded)
            .count();
        let rejected = reports.len() - loaded;
        let report = ReloadReport {
            loaded,
            rejected,
            plugins: registry.plugins(),
            reports,
        };
        self.registry.store(Arc::new(registry));
        report
    }

    pub fn plugins(&self) -> Vec<PluginInfo> {
        self.registry.load().plugins()
    }

    pub fn reports(&self) -> Vec<PluginLoadReport> {
        self.registry.load().reports().to_vec()
    }

    pub fn get(&self, id: &ProviderId) -> Option<PluginInfo> {
        self.registry.load().get(id).map(|entry| entry.info.clone())
    }

    /// 注册表条目快照（含配方与上限），供设置页的插件详情展示。
    pub fn entry(&self, id: &ProviderId) -> Option<RegistryEntry> {
        self.registry.load().get(id).cloned()
    }

    pub async fn lookup(
        &self,
        id: &ProviderId,
        word: &str,
    ) -> Result<DictEntry, DictProviderError> {
        let entry = self.registry.load().get(id).cloned();
        let Some(entry) = entry else {
            return Err(DictProviderError::Parse(format!("词典源未注册: {id}")));
        };
        let Some(recipe) = entry.recipe.lookup.as_ref() else {
            return Err(DictProviderError::Parse(format!(
                "词典源未提供 lookup 配方: {id}"
            )));
        };
        let limits = self.entry_limits(&entry);
        let request = recipe
            .request
            .build(word, &entry.info.hosts, limits.max_body_bytes)?;
        let response = self
            .transport
            .execute(request)
            .await
            .map_err(TransportError::into_provider_error)?;
        recipe.execute(&response.body, word, &entry.info, &limits)
    }

    /// 依次尝试启用的建议插件；失败或空结果继续下一个，全部失败返回空。
    pub async fn suggest(&self, word: &str, order: &[ProviderId]) -> Vec<DictSuggestion> {
        for id in order {
            let entry = self.registry.load().get(id).cloned();
            let Some(entry) = entry else {
                continue;
            };
            if !entry.info.capabilities.contains(&Capability::Suggest) {
                continue;
            }
            let Some(recipe) = entry.recipe.suggest.as_ref() else {
                continue;
            };
            let limits = self.entry_limits(&entry);
            let request = match recipe
                .request
                .build(word, &entry.info.hosts, limits.max_body_bytes)
            {
                Ok(request) => request,
                Err(error) => {
                    tracing::warn!(provider = %id, error = %error, "词典建议请求构造失败");
                    continue;
                }
            };
            let response = match self.transport.execute(request).await {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(provider = %id, error = %error, "词典建议请求失败");
                    continue;
                }
            };
            match recipe.execute(&response.body, word, &limits) {
                Ok(items) if !items.is_empty() => return items,
                Ok(_) => continue,
                Err(error) => {
                    tracing::warn!(provider = %id, error = %error, "词典建议解析失败");
                }
            }
        }
        Vec::new()
    }

    fn entry_limits(&self, entry: &RegistryEntry) -> EngineLimits {
        let mut limits = self.limits.clone();
        limits.max_body_bytes = limits.max_body_bytes.min(entry.limits.max_body_bytes);
        limits.max_definitions = limits.max_definitions.min(entry.limits.max_definitions);
        limits.max_examples = limits.max_examples.min(entry.limits.max_examples);
        limits
    }
}
