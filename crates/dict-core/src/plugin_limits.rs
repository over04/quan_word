use serde::Deserialize;

/// 插件包声明的上限（只能比引擎硬上限更小）。
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PluginLimits {
    pub max_body_bytes: usize,
    pub max_definitions: usize,
    pub max_examples: usize,
}

impl Default for PluginLimits {
    fn default() -> Self {
        Self {
            max_body_bytes: 1_048_576,
            max_definitions: 20,
            max_examples: 5,
        }
    }
}
