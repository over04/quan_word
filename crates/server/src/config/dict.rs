use crate::business::dict::provider_id::DictProviderId;

/// 词典抓词配置：源开关、超时、缓存/熔断参数与输入长度上限。
///
/// 缺省值面向公网抓词：6 秒超时、7 天正向缓存、10 分钟负缓存、8 路并发、
/// 连续 3 次失败熔断 60 秒。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DictConfig {
    /// 是否启用词典抓词接口（关闭后 /api/dict/lookup 返回 400）
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 启用的词典源（同时也是聚合结果的顺序）
    #[serde(default = "default_providers")]
    pub providers: Vec<DictProviderId>,
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

fn default_providers() -> Vec<DictProviderId> {
    DictProviderId::ALL.to_vec()
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
            providers: default_providers(),
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
    /// 启动期校验：数值合法性与源列表非空（消息为中文，直接进启动错误）。
    pub fn validate(&self) -> Result<(), String> {
        if self.enabled && self.providers.is_empty() {
            return Err(
                "dict.providers 不能为空（dict.enabled 为 true 时至少启用一个词典源）".into(),
            );
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
        assert_eq!(cfg.providers.len(), 8);
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
        ];
        for cfg in invalid {
            assert!(cfg.validate().is_err(), "非法数值应被拒绝: {cfg:?}");
        }
    }

    #[test]
    fn rejects_empty_providers_when_enabled() {
        let cfg = DictConfig {
            providers: Vec::new(),
            ..DictConfig::default()
        };
        assert!(cfg.validate().is_err());
        let disabled = DictConfig {
            enabled: false,
            providers: Vec::new(),
            ..DictConfig::default()
        };
        assert!(disabled.validate().is_ok(), "关闭词典时不要求源列表非空");
    }
}
