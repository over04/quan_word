use serde::Deserialize;

use crate::capability::Capability;
use crate::engine_limits::EngineLimits;
use crate::plugin_engine::PluginEngine;
use crate::plugin_error::PluginError;
use crate::plugin_manifest::PluginManifest;
use crate::plugin_source::PluginSource;
use crate::recipe::validate::validate_host_pattern;
use crate::recipe::Recipe;

/// 插件包：清单 + 配方（`data/plugins/<id>.json` 的根结构）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginPackage {
    pub manifest: PluginManifest,
    pub recipe: Recipe,
}

impl PluginPackage {
    /// 解析并校验插件包；错误消息可直接写日志/返回管理接口。
    pub fn parse(source: &PluginSource, limits: &EngineLimits) -> Result<Self, PluginError> {
        if source.bytes.len() > limits.max_package_bytes {
            return Err(PluginError::TooLarge {
                max: limits.max_package_bytes,
            });
        }
        let text = std::str::from_utf8(&source.bytes).map_err(|_| PluginError::NotUtf8)?;
        let package: Self =
            serde_json::from_str(text).map_err(|error| PluginError::Json(error.to_string()))?;
        package.validate(limits)?;
        Ok(package)
    }

    fn validate(&self, limits: &EngineLimits) -> Result<(), PluginError> {
        let manifest = &self.manifest;
        if manifest.api_version != 1 {
            return Err(PluginError::UnsupportedApiVersion(manifest.api_version));
        }
        if manifest.engine != PluginEngine::Recipe {
            return Err(PluginError::UnsupportedEngine("wasm".into()));
        }
        if manifest.label.trim().is_empty() {
            return Err(PluginError::InvalidRecipe("label 不能为空".into()));
        }
        if manifest.label.chars().count() > 64 {
            return Err(PluginError::InvalidRecipe("label 过长".into()));
        }
        if let Some(short) = &manifest.short_label {
            if short.trim().is_empty() || short.chars().count() > 16 {
                return Err(PluginError::InvalidRecipe(
                    "short_label 不能为空且不超过 16 字符".into(),
                ));
            }
        }
        if manifest.version.trim().is_empty() || manifest.version.chars().count() > 32 {
            return Err(PluginError::InvalidRecipe(
                "version 不能为空且不超过 32 字符".into(),
            ));
        }
        if let Some(description) = &manifest.description {
            if description.chars().count() > 200 {
                return Err(PluginError::InvalidRecipe("description 过长".into()));
            }
        }
        if manifest.capabilities.is_empty() {
            return Err(PluginError::InvalidCapability("至少声明一种能力".into()));
        }
        for (index, capability) in manifest.capabilities.iter().enumerate() {
            if manifest.capabilities[..index].contains(capability) {
                return Err(PluginError::InvalidCapability("能力重复声明".into()));
            }
        }
        if manifest.hosts.is_empty() {
            return Err(PluginError::InvalidHost("hosts 不能为空".into()));
        }
        for host in &manifest.hosts {
            validate_host_pattern(host)?;
        }
        if manifest.limits.max_body_bytes == 0
            || manifest.limits.max_body_bytes > limits.max_body_bytes
        {
            return Err(PluginError::InvalidRecipe(format!(
                "max_body_bytes 必须在 1..={} 之间",
                limits.max_body_bytes
            )));
        }
        if manifest.limits.max_definitions == 0
            || manifest.limits.max_definitions > limits.max_definitions
        {
            return Err(PluginError::InvalidRecipe(format!(
                "max_definitions 必须在 1..={} 之间",
                limits.max_definitions
            )));
        }
        if manifest.limits.max_examples == 0 || manifest.limits.max_examples > limits.max_examples {
            return Err(PluginError::InvalidRecipe(format!(
                "max_examples 必须在 1..={} 之间",
                limits.max_examples
            )));
        }
        let has_lookup = manifest.capabilities.contains(&Capability::Lookup);
        let has_suggest = manifest.capabilities.contains(&Capability::Suggest);
        if has_lookup && self.recipe.lookup.is_none() {
            return Err(PluginError::MissingRecipe("lookup".into()));
        }
        if has_suggest && self.recipe.suggest.is_none() {
            return Err(PluginError::MissingRecipe("suggest".into()));
        }
        if !has_lookup && self.recipe.lookup.is_some() {
            return Err(PluginError::InvalidCapability(
                "声明 lookup 配方但未声明 lookup 能力".into(),
            ));
        }
        if !has_suggest && self.recipe.suggest.is_some() {
            return Err(PluginError::InvalidCapability(
                "声明 suggest 配方但未声明 suggest 能力".into(),
            ));
        }
        self.recipe
            .validate(&manifest.hosts, manifest.limits.max_body_bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::PluginPackage;
    use crate::engine_limits::EngineLimits;
    use crate::plugin_error::PluginError;
    use crate::plugin_source::PluginSource;

    fn package(body: &str) -> Result<PluginPackage, PluginError> {
        PluginPackage::parse(
            &PluginSource::new("test.json", body.as_bytes().to_vec()),
            &EngineLimits::default(),
        )
    }

    #[test]
    fn parses_valid_package() {
        let pkg = package(
            r#"{"manifest":{"api_version":1,"id":"demo","label":"示例","version":"1.0.0","engine":"recipe","capabilities":["lookup"],"hosts":["example.com"]},"recipe":{"lookup":{"request":{"method":"get","url":"https://example.com/{word}"},"definitions":[{"kind":"plain_text","from":{"text":"释义"},"separators":[";"]}]}}}"#,
        )
        .unwrap();
        assert_eq!(pkg.manifest.id.as_str(), "demo");
        assert_eq!(pkg.manifest.short_label, None);
    }

    #[test]
    fn rejects_unknown_fields_and_bad_hosts() {
        let unknown = r#"{"manifest":{"api_version":1,"id":"demo","label":"示例","version":"1.0.0","engine":"recipe","capabilities":["lookup"],"hosts":["example.com"],"extra":1},"recipe":{"lookup":{"request":{"method":"get","url":"https://example.com/{word}"},"definitions":[{"kind":"plain_text","from":{"text":"释义"},"separators":[";"]}]}}}"#;
        assert!(matches!(
            package(unknown).unwrap_err(),
            PluginError::Json(_)
        ));

        let bad_host = r#"{"manifest":{"api_version":1,"id":"demo","label":"示例","version":"1.0.0","engine":"recipe","capabilities":["lookup"],"hosts":["example.com"]},"recipe":{"lookup":{"request":{"method":"get","url":"https://evil.com/{word}"},"definitions":[{"kind":"plain_text","from":{"text":"释义"},"separators":[";"]}]}}}"#;
        assert!(matches!(
            package(bad_host).unwrap_err(),
            PluginError::HostNotAllowed(_)
        ));
    }

    #[test]
    fn rejects_wasm_and_missing_recipe() {
        let wasm = r#"{"manifest":{"api_version":1,"id":"demo","label":"示例","version":"1.0.0","engine":"wasm","capabilities":["lookup"],"hosts":["example.com"]},"recipe":{"lookup":{"request":{"method":"get","url":"https://example.com/{word}"},"definitions":[{"kind":"plain_text","from":{"text":"释义"},"separators":[";"]}]}}}"#;
        assert!(matches!(
            package(wasm).unwrap_err(),
            PluginError::UnsupportedEngine(_)
        ));

        let missing = r#"{"manifest":{"api_version":1,"id":"demo","label":"示例","version":"1.0.0","engine":"recipe","capabilities":["lookup"],"hosts":["example.com"]},"recipe":{"suggest":{"request":{"method":"get","url":"https://example.com/{word}"},"items":{"from":{"all":"data[*]"},"fields":{"word":{"path":"word"},"summary":{"path":"summary"}}}}}}"#;
        assert!(matches!(
            package(missing).unwrap_err(),
            PluginError::MissingRecipe(_)
        ));
    }
}
