use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;

use super::breaker::DictBreakers;
use super::cache::DictCache;
use super::http::DictHttp;
use super::provider::DictProvider;
use super::provider_id::DictProviderId;
use super::singleflight::DictSingleflight;
use super::suggest_cache::SuggestCache;
use super::suggest_limiter::SuggestRateLimiter;
use crate::config::dict::DictConfig;

/// 词典域运行时状态：配置 + 源实例 + HTTP 客户端 + 缓存/熔断/单飞/并发闸门。
///
/// 由 `router::build` 构造一次，随后以 `Arc` 挂到 `AppState`；
/// 缓存清理任务由 `spawn_cleaner` 启动（随进程结束终止）。
pub struct DictState {
    pub config: DictConfig,
    /// 已注册的源实例（顺序 = 配置里的 providers 顺序）
    pub providers: Vec<Arc<dyn DictProvider>>,
    pub http: DictHttp,
    pub cache: DictCache,
    pub breakers: DictBreakers,
    pub singleflight: DictSingleflight,
    pub suggest_cache: SuggestCache,
    /// 建议接口的按 IP 限流（无鉴权 + 扇出上游，必须兜一层）
    pub suggest_limiter: SuggestRateLimiter,
    /// 上游并发上限（semaphore permit 在每个请求内短持有）
    pub semaphore: Arc<Semaphore>,
}

impl DictState {
    pub fn new(config: &DictConfig) -> Self {
        let mut ids: Vec<DictProviderId> = Vec::new();
        for id in &config.providers {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        Self {
            config: config.clone(),
            providers: ids.into_iter().map(super::providers::build).collect(),
            http: DictHttp::new(config.timeout_ms),
            cache: DictCache::new(
                config.cache_ttl_secs,
                config.negative_ttl_secs,
                config.cache_cap,
            ),
            breakers: DictBreakers::new(config.breaker_threshold, config.breaker_cooldown_secs),
            singleflight: DictSingleflight::new(),
            suggest_cache: SuggestCache::new(config.cache_ttl_secs, config.cache_cap),
            suggest_limiter: SuggestRateLimiter::new(),
            semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
        }
    }

    /// 启动缓存后台清理：按 `cache_cleanup_secs` 扫描并清除过期条目。
    pub fn spawn_cleaner(self: &Arc<Self>) {
        let state = Arc::clone(self);
        let interval = Duration::from_secs(state.config.cache_cleanup_secs.max(1));
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                state.cache.cleanup(now);
                state.suggest_cache.cleanup(now);
            }
        });
    }

    /// 词典功能是否启用（关闭时抓词接口返回 400，源列表仍可查）。
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// 按 id 取已注册的源实例。
    pub fn provider(&self, id: DictProviderId) -> Option<&Arc<dyn DictProvider>> {
        self.providers.iter().find(|provider| provider.id() == id)
    }
}
