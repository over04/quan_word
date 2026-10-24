use std::collections::HashMap;

use crate::engine_limits::EngineLimits;
use crate::plugin_info::PluginInfo;
use crate::plugin_load_report::PluginLoadReport;
use crate::plugin_package::PluginPackage;
use crate::plugin_source::PluginSource;
use crate::plugin_status::PluginStatus;
use crate::provider_id::ProviderId;
use crate::registry_entry::RegistryEntry;

/// 已装载插件集合：外部目录按文件名排序；同 id 全部拒绝；数量上限内装载。
#[derive(Debug, Clone, Default)]
pub struct PluginRegistry {
    entries: Vec<RegistryEntry>,
    reports: Vec<PluginLoadReport>,
}

impl PluginRegistry {
    pub fn build(mut sources: Vec<PluginSource>, limits: &EngineLimits) -> Self {
        sources.sort_by(|a, b| a.file.cmp(&b.file));
        let parsed: Vec<(
            PluginSource,
            Result<PluginPackage, crate::plugin_error::PluginError>,
        )> = sources
            .into_iter()
            .map(|source| {
                let result = PluginPackage::parse(&source, limits);
                (source, result)
            })
            .collect();
        let mut reasons: Vec<Option<String>> = vec![None; parsed.len()];

        let mut counts: HashMap<String, usize> = HashMap::new();
        for (_, result) in &parsed {
            if let Ok(package) = result {
                *counts.entry(package.manifest.id.to_string()).or_default() += 1;
            }
        }
        for (index, (_, result)) in parsed.iter().enumerate() {
            if let Ok(package) = result {
                if counts
                    .get(package.manifest.id.as_str())
                    .copied()
                    .unwrap_or(0)
                    > 1
                {
                    reasons[index] = Some(format!("插件 id 重复: {}", package.manifest.id));
                }
            }
        }

        let mut entries: Vec<RegistryEntry> = Vec::new();
        for (index, (source, result)) in parsed.iter().enumerate() {
            if reasons[index].is_some() {
                continue;
            }
            let Ok(package) = result else {
                continue;
            };
            if entries.len() >= limits.max_plugins {
                reasons[index] = Some("插件数量超过上限".into());
                continue;
            }
            entries.push(entry(package, source));
        }

        let reports = parsed
            .iter()
            .enumerate()
            .map(
                |(index, (source, result))| match (result, &reasons[index]) {
                    (Ok(package), None) => PluginLoadReport {
                        file: source.file.clone(),
                        id: Some(package.manifest.id.clone()),
                        status: PluginStatus::Loaded,
                        error: None,
                    },
                    (Ok(package), Some(reason)) => PluginLoadReport {
                        file: source.file.clone(),
                        id: Some(package.manifest.id.clone()),
                        status: PluginStatus::Rejected,
                        error: Some(reason.clone()),
                    },
                    (Err(error), _) => PluginLoadReport {
                        file: source.file.clone(),
                        id: None,
                        status: PluginStatus::Rejected,
                        error: Some(error.to_string()),
                    },
                },
            )
            .collect();

        Self { entries, reports }
    }

    pub fn get(&self, id: &ProviderId) -> Option<&RegistryEntry> {
        self.entries.iter().find(|entry| entry.info.id == *id)
    }

    pub fn plugins(&self) -> Vec<PluginInfo> {
        self.entries
            .iter()
            .map(|entry| entry.info.clone())
            .collect()
    }

    pub fn reports(&self) -> &[PluginLoadReport] {
        &self.reports
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn entry(package: &PluginPackage, source: &PluginSource) -> RegistryEntry {
    RegistryEntry {
        info: PluginInfo::from_manifest(&package.manifest),
        recipe: package.recipe.clone(),
        limits: package.manifest.limits,
        file: source.file.clone(),
    }
}
#[cfg(test)]
mod tests {
    use super::PluginRegistry;
    use crate::engine_limits::EngineLimits;
    use crate::plugin_source::PluginSource;
    use crate::plugin_status::PluginStatus;
    use crate::provider_id::ProviderId;

    fn source(file: &str, id: &str, version: &str) -> PluginSource {
        let body = format!(
            r#"{{"manifest":{{"api_version":1,"id":"{id}","label":"{id}","version":"{version}","engine":"recipe","capabilities":["lookup"],"hosts":["example.com"]}},"recipe":{{"lookup":{{"request":{{"method":"get","url":"https://example.com/{id}"}},"definitions":[{{"kind":"plain_text","from":{{"text":"释义"}},"separators":[";"]}}]}}}}}}"#
        );
        PluginSource::new(file, body.into_bytes())
    }

    fn id(raw: &str) -> ProviderId {
        ProviderId::new(raw).unwrap()
    }

    #[test]
    fn loads_in_filename_order() {
        let registry = PluginRegistry::build(
            vec![
                source("z.json", "gamma", "1.0.0"),
                source("a.json", "alpha", "1.0.0"),
                source("b.json", "beta", "1.0.0"),
            ],
            &EngineLimits::default(),
        );
        let ids: Vec<String> = registry
            .plugins()
            .into_iter()
            .map(|plugin| plugin.id.to_string())
            .collect();
        assert_eq!(ids, vec!["alpha", "beta", "gamma"]);
        assert_eq!(registry.reports().len(), 3);
        assert!(registry
            .reports()
            .iter()
            .all(|report| report.status == PluginStatus::Loaded));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let registry = PluginRegistry::build(
            vec![
                source("a.json", "alpha", "1.0.0"),
                source("z.json", "alpha", "2.0.0"),
            ],
            &EngineLimits::default(),
        );
        assert!(registry.is_empty());
        assert_eq!(registry.reports().len(), 2);
        assert!(registry
            .reports()
            .iter()
            .all(|report| report.status == PluginStatus::Rejected));
    }

    #[test]
    fn enforces_plugin_cap() {
        let limits = EngineLimits {
            max_plugins: 1,
            ..EngineLimits::default()
        };
        let registry = PluginRegistry::build(
            vec![
                source("a.json", "alpha", "1.0.0"),
                source("b.json", "beta", "1.0.0"),
            ],
            &limits,
        );
        assert_eq!(registry.plugins().len(), 1);
        assert_eq!(registry.reports()[1].status, PluginStatus::Rejected);
    }

    #[test]
    fn reports_broken_package_without_panicking() {
        let registry = PluginRegistry::build(
            vec![PluginSource::new("broken.json", b"{".to_vec())],
            &EngineLimits::default(),
        );
        assert!(registry.is_empty());
        assert_eq!(registry.reports().len(), 1);
        assert_eq!(registry.reports()[0].status, PluginStatus::Rejected);
        assert!(registry.reports()[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("JSON"));
    }

    #[test]
    fn registry_lookup_by_id() {
        let registry = PluginRegistry::build(
            vec![source("a.json", "alpha", "1.0.0")],
            &EngineLimits::default(),
        );
        assert_eq!(registry.get(&id("alpha")).unwrap().info.version, "1.0.0");
        assert!(registry.get(&id("missing")).is_none());
    }
}
