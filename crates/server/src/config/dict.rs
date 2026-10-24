use dict_core::provider_id::ProviderId;

/// 词典抓词配置：插件目录、源开关、超时、缓存/熔断参数与输入长度上限。
///
/// 缺省值面向公网抓词：6 秒超时、7 天正向缓存、10 分钟负缓存、8 路并发、
/// 连续 3 次失败熔断 60 秒；`providers: []` 表示启用全部已安装插件。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DictConfig {
    /// 是否启用词典抓词接口（关闭后 /api/dict/lookup 返回 400）
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 启用的词典源 id（空 = 全部已安装插件；非空时同时决定聚合顺序）
    #[serde(default)]
    pub providers: Vec<String>,
    /// 插件目录（不存在或为空时启动后无可用源，可从设置页导入）
    #[serde(default = "default_plugins_dir")]
    pub plugins_dir: String,
    /// 插件目录轮询间隔（秒）；0 = 不轮询，仅启动时装载
    #[serde(default = "default_plugins_poll_secs")]
    pub plugins_poll_secs: u64,
    /// 插件数量上限
    #[serde(default = "default_max_plugins")]
    pub max_plugins: usize,
    /// 单次上游请求超时（毫秒）
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// 正向结果缓存有效期（秒）
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
    /// 负向结果（未收录）缓存有效期（秒）
    #[serde(default = "default_negative_ttl_secs")]
    pub negative_ttl_secs: u64,
    /// 缓存条目上限（条）：超出先清过期、再淘汰最旧
    #[serde(default = "default_cache_cap")]
    pub cache_cap: usize,
    /// 后台清理任务扫描间隔（秒）
    #[serde(default = "default_cache_cleanup_secs")]
    pub cache_cleanup_secs: u64,
    /// 同时进行中的上游请求上限
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
    /// 连续失败多少次后熔断该源
    #[serde(default = "default_breaker_threshold")]
    pub breaker_threshold: u32,
    /// 熔断冷却时长（秒）
    #[serde(default = "default_breaker_cooldown_secs")]
    pub breaker_cooldown_secs: u64,
    /// 单词最大字符数（1..=128）
    #[serde(default = "default_max_word_len")]
    pub max_word_len: usize,
}

const fn default_enabled() -> bool {
    true
}

fn default_plugins_dir() -> String {
    "./plugins".to_string()
}

const fn default_plugins_poll_secs() -> u64 {
    5
}

const fn default_max_plugins() -> usize {
    32
}

const fn default_timeout_ms() -> u64 {
    6000
}

const fn default_cache_ttl_secs() -> u64 {
    604_800 // 7 天
}

const fn default_negative_ttl_secs() -> u64 {
    600 // 10 分钟
}

const fn default_cache_cap() -> usize {
    2048
}

const fn default_cache_cleanup_secs() -> u64 {
    300
}

const fn default_max_concurrency() -> usize {
    8
}

const fn default_breaker_threshold() -> u32 {
    3
}

const fn default_breaker_cooldown_secs() -> u64 {
    60
}

const fn default_max_word_len() -> usize {
    64
}

impl Default for DictConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            providers: Vec::new(),
            plugins_dir: default_plugins_dir(),
            plugins_poll_secs: default_plugins_poll_secs(),
            max_plugins: default_max_plugins(),
            timeout_ms: default_timeout_ms(),
            cache_ttl_secs: default_cache_ttl_secs(),
            negative_ttl_secs: default_negative_ttl_secs(),
            cache_cap: default_cache_cap(),
            cache_cleanup_secs: default_cache_cleanup_secs(),
            max_concurrency: default_max_concurrency(),
            breaker_threshold: default_breaker_threshold(),
            breaker_cooldown_secs: default_breaker_cooldown_secs(),
            max_word_len: default_max_word_len(),
        }
    }
}

impl DictConfig {
    /// 启动期校验：数值合法性、源 id 格式与插件目录（消息为中文，直接进启动错误）。
    pub fn validate(&self) -> Result<(), String> {
        for raw in &self.providers {
            ProviderId::new(raw.clone())
                .map_err(|error| format!("dict.providers 含非法源 id: {error}"))?;
        }
        if self.plugins_dir.trim().is_empty() {
            return Err("dict.plugins_dir 不能为空".into());
        }
        if !(1..=256).contains(&self.max_plugins) {
            return Err(format!(
                "dict.max_plugins 必须在 1..=256 之间（当前 {}）",
                self.max_plugins
            ));
        }
        if !(500..=30_000).contains(&self.timeout_ms) {
            return Err(format!(
                "dict.timeout_ms 必须在 500..=30000 之间（当前 {}）",
                self.timeout_ms
            ));
        }
        if !(1..=128).contains(&self.max_word_len) {
            return Err(format!(
                "dict.max_word_len 必须在 1..=128 之间（当前 {}）",
                self.max_word_len
            ));
        }
        require_positive("cache_ttl_secs", self.cache_ttl_secs)?;
        require_positive("negative_ttl_secs", self.negative_ttl_secs)?;
        require_positive("cache_cleanup_secs", self.cache_cleanup_secs)?;
        require_positive("breaker_cooldown_secs", self.breaker_cooldown_secs)?;
        require_positive("cache_cap", self.cache_cap as u64)?;
        require_positive("max_concurrency", self.max_concurrency as u64)?;
        require_positive("breaker_threshold", u64::from(self.breaker_threshold))?;
        Ok(())
    }
}

/// 数值配置必须大于 0（`name` 为配置项名，直接进错误消息）。
fn require_positive(name: &str, value: u64) -> Result<(), String> {
    if value == 0 {
        return Err(format!("dict.{name} 必须大于 0"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DictConfig;

    #[test]
    fn defaults_match_documented_values() {
        let cfg = DictConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.providers.is_empty(), "缺省启用全部已安装插件");
        assert_eq!(cfg.plugins_dir, "./plugins");
        assert_eq!(cfg.plugins_poll_secs, 5);
        assert_eq!(cfg.max_plugins, 32);
        assert_eq!(cfg.timeout_ms, 6000);
        assert_eq!(cfg.cache_ttl_secs, 604_800);
        assert_eq!(cfg.negative_ttl_secs, 600);
        assert_eq!(cfg.cache_cap, 2048);
        assert_eq!(cfg.cache_cleanup_secs, 300);
        assert_eq!(cfg.max_concurrency, 8);
        assert_eq!(cfg.breaker_threshold, 3);
        assert_eq!(cfg.breaker_cooldown_secs, 60);
        assert_eq!(cfg.max_word_len, 64);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_invalid_numbers() {
        let invalid = [
            DictConfig {
                timeout_ms: 100,
                ..DictConfig::default()
            },
            DictConfig {
                timeout_ms: 60_000,
                ..DictConfig::default()
            },
            DictConfig {
                max_word_len: 0,
                ..DictConfig::default()
            },
            DictConfig {
                max_word_len: 129,
                ..DictConfig::default()
            },
            DictConfig {
                cache_cap: 0,
                ..DictConfig::default()
            },
            DictConfig {
                max_concurrency: 0,
                ..DictConfig::default()
            },
            DictConfig {
                cache_ttl_secs: 0,
                ..DictConfig::default()
            },
            DictConfig {
                negative_ttl_secs: 0,
                ..DictConfig::default()
            },
            DictConfig {
                breaker_threshold: 0,
                ..DictConfig::default()
            },
            DictConfig {
                breaker_cooldown_secs: 0,
                ..DictConfig::default()
            },
            DictConfig {
                max_plugins: 0,
                ..DictConfig::default()
            },
            DictConfig {
                plugins_dir: "  ".into(),
                ..DictConfig::default()
            },
        ];
        for cfg in invalid {
            assert!(cfg.validate().is_err(), "非法数值应被拒绝: {cfg:?}");
        }
    }

    #[test]
    fn accepts_empty_providers_and_validates_ids() {
        let cfg = DictConfig {
            providers: Vec::new(),
            ..DictConfig::default()
        };
        assert!(cfg.validate().is_ok(), "空源列表表示全部已安装插件");

        let valid = DictConfig {
            providers: vec!["youdao".into(), "free_dict".into()],
            ..DictConfig::default()
        };
        assert!(valid.validate().is_ok());

        let invalid = DictConfig {
            providers: vec!["Youdao".into()],
            ..DictConfig::default()
        };
        assert!(invalid.validate().is_err());
    }
}
