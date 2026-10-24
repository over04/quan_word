use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use dict_core::engine_limits::EngineLimits;
use dict_core::plugin_info::PluginInfo;
use dict_core::plugin_status::PluginStatus;
use dict_core::provider_id::ProviderId;
use dict_core::reload_report::ReloadReport;
use dict_core::search_engine::SearchEngine;
use dict_core::transport::SearchTransport;
use parking_lot::{Mutex as SyncMutex, RwLock};
use tokio::sync::{Mutex, Semaphore};

use super::breaker::DictBreakers;
use super::cache::DictCache;
use super::plugin_prefs::PluginPrefs;
use super::plugin_store::PluginStore;
use super::singleflight::DictSingleflight;
use super::suggest_cache::SuggestCache;
use super::suggest_limiter::SuggestRateLimiter;
use crate::config::dict::DictConfig;

/// 词典域运行时状态：配置 + 插件引擎 + 缓存/熔断/单飞/并发闸门。
///
/// 由 `router::build` 构造一次，随后以 `Arc` 挂到 `AppState`；
/// 缓存清理任务由 `spawn_cleaner` 启动，插件轮询由 `spawn_poller` 启动。
pub struct DictState {
    pub config: DictConfig,
    /// 插件引擎（注册表可原子替换；进行中的请求持有旧 `Arc`）
    pub engine: Arc<SearchEngine>,
    /// 运行时插件目录
    pub plugin_store: PluginStore,
    /// 配置显式启用的源；空 = 全部已安装插件
    enabled: Vec<ProviderId>,
    /// 运行时启停覆盖（持久化在插件目录的隐藏状态文件）
    prefs: RwLock<PluginPrefs>,
    /// 插件目录指纹，轮询时只在变化后重载
    plugin_signature: SyncMutex<String>,
    /// 重载串行化，避免并发 reload 交叉替换
    pub reload_lock: Mutex<()>,
    pub cache: DictCache,
    pub breakers: DictBreakers,
    pub singleflight: DictSingleflight,
    pub suggest_cache: SuggestCache,
    /// 建议接口的按 IP 限流（会扇出上游，额外兜一层；鉴权仍由全局中间件负责）
    pub suggest_limiter: SuggestRateLimiter,
    /// 上游并发上限（semaphore permit 在每个请求内短持有）
    pub semaphore: Arc<Semaphore>,
}

impl DictState {
    /// 生产构造：只装载插件目录；空目录允许启动，由设置页导入。
    pub fn new(config: &DictConfig, transport: Arc<dyn SearchTransport>) -> Result<Self> {
        let limits = EngineLimits {
            max_plugins: config.max_plugins,
            ..EngineLimits::default()
        };
        let engine = Arc::new(SearchEngine::new(transport, limits));
        let mut state = Self::from_engine(config, engine)?;
        state.prefs = RwLock::new(PluginPrefs::load(state.plugin_store.prefs_path()));
        let report = state.load_plugins();
        tracing::info!(
            loaded = report.loaded,
            rejected = report.rejected,
            "词典插件装载完成"
        );
        if config.enabled {
            if let Some(missing) = state
                .enabled
                .iter()
                .find(|id| state.engine.get(id).is_none())
            {
                anyhow::bail!("dict.providers 含未安装的词典源: {missing}");
            }
        }
        Ok(state)
    }

    /// 用已装载的引擎构造状态（测试 / 嵌入场景）；不读取插件目录。
    pub fn from_engine(config: &DictConfig, engine: Arc<SearchEngine>) -> Result<Self> {
        let enabled = config
            .providers
            .iter()
            .map(|raw| {
                ProviderId::new(raw.clone())
                    .map_err(|error| anyhow::anyhow!("dict.providers 含非法源 id: {error}"))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            config: config.clone(),
            engine,
            plugin_store: PluginStore::new(config),
            enabled,
            prefs: RwLock::new(PluginPrefs::memory()),
            plugin_signature: SyncMutex::new(String::new()),
            reload_lock: Mutex::new(()),
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
        })
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

    /// 外部插件目录轮询（`plugins_poll_secs = 0` 时不启动）。
    pub fn spawn_poller(self: &Arc<Self>) {
        let secs = self.config.plugins_poll_secs;
        if secs == 0 {
            return;
        }
        let state = Arc::clone(self);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(secs));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                let signature = state.plugin_store.signature();
                if signature == *state.plugin_signature.lock() {
                    continue;
                }
                let report = state.reload_plugins().await;
                tracing::info!(
                    loaded = report.loaded,
                    rejected = report.rejected,
                    "词典插件目录变化，已自动重载"
                );
            }
        });
    }

    /// 串行重载插件；成功后清空词典缓存、建议缓存与熔断状态。
    pub async fn reload_plugins(&self) -> ReloadReport {
        let _guard = self.reload_lock.lock().await;
        let report = self.load_plugins();
        self.cache.clear();
        self.suggest_cache.clear();
        self.breakers.clear();
        report
    }

    /// 同步装载（启动期使用；不持锁，调用方保证只执行一次）。
    pub fn load_plugins(&self) -> ReloadReport {
        let signature = self.plugin_store.signature();
        let collected = self.plugin_store.collect();
        let mut report = self.engine.load(collected.sources);
        if !collected.failures.is_empty() {
            report.rejected += collected.failures.len();
            report.reports.extend(collected.failures);
        }
        for item in &report.reports {
            if item.status == PluginStatus::Rejected {
                tracing::warn!(
                    file = %item.file,
                    error = item.error.as_deref().unwrap_or(""),
                    "词典插件被拒绝"
                );
            }
        }
        *self.plugin_signature.lock() = signature;
        report
    }

    /// 词典功能是否启用（关闭时抓词接口返回 400，源列表仍可查）。
    pub fn enabled(&self) -> bool {
        self.config.enabled
    }

    /// 当前启用的源：配置顺序优先，其次是被手动打开的外部源。
    pub fn enabled_ids(&self) -> Vec<ProviderId> {
        let mut ids = Vec::new();
        for id in &self.enabled {
            if self.is_enabled(id) {
                ids.push(id.clone());
            }
        }
        for plugin in self.engine.plugins() {
            if self.is_enabled(&plugin.id) && !ids.contains(&plugin.id) {
                ids.push(plugin.id);
            }
        }
        ids
    }

    /// 源是否启用：配置是默认值，运行时启停偏好优先。
    pub fn is_enabled(&self, id: &ProviderId) -> bool {
        if self.engine.get(id).is_none() {
            return false;
        }
        let prefs = self.prefs.read();
        if prefs.is_disabled(id) {
            return false;
        }
        self.enabled.is_empty() || self.enabled.contains(id) || prefs.is_extra_enabled(id)
    }

    /// 更新运行时启停偏好并落盘；插件不存在时返回 None。
    pub fn set_enabled(&self, id: &ProviderId, enabled: bool) -> Option<PluginInfo> {
        let plugin = self.engine.get(id)?;
        let outside_config = !self.enabled.is_empty() && !self.enabled.contains(id);
        self.prefs.write().set_enabled(id, enabled, outside_config);
        Some(plugin)
    }

    /// 已安装插件的公开信息。
    pub fn plugin(&self, id: &ProviderId) -> Option<PluginInfo> {
        self.engine.get(id)
    }
}
