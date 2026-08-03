/// 批量导入配置：解析上限与预览会话缓存生命周期。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImportConfig {
    /// 单次导入数据行数上限（不含表头）
    #[serde(default = "default_max_rows")]
    pub max_rows: usize,
    /// 预览会话有效期（秒）：超时未导入自动清理
    #[serde(default = "default_cache_ttl_secs")]
    pub cache_ttl_secs: u64,
    /// 预览会话缓存上限（条）：超出整体清空
    #[serde(default = "default_cache_cap")]
    pub cache_cap: usize,
    /// 后台清理任务扫描间隔（秒）
    #[serde(default = "default_cache_cleanup_secs")]
    pub cache_cleanup_secs: u64,
}

const fn default_max_rows() -> usize {
    5000
}

const fn default_cache_ttl_secs() -> u64 {
    1800 // 30 分钟
}

const fn default_cache_cap() -> usize {
    16
}

const fn default_cache_cleanup_secs() -> u64 {
    60
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            max_rows: default_max_rows(),
            cache_ttl_secs: default_cache_ttl_secs(),
            cache_cap: default_cache_cap(),
            cache_cleanup_secs: default_cache_cleanup_secs(),
        }
    }
}
