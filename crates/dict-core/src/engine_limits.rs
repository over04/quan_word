/// 引擎硬上限（插件 manifest 只能调低，不能突破）。
#[derive(Debug, Clone)]
pub struct EngineLimits {
    pub max_plugins: usize,
    pub max_package_bytes: usize,
    pub max_body_bytes: usize,
    pub max_definitions: usize,
    pub max_examples: usize,
    pub max_suggestions: usize,
    pub max_meaning_chars: usize,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_plugins: 32,
            max_package_bytes: 262_144,
            max_body_bytes: 1_048_576,
            max_definitions: 20,
            max_examples: 5,
            max_suggestions: 24,
            max_meaning_chars: 300,
        }
    }
}
