use std::time::Instant;

use super::cached_lookup::CachedLookup;

/// 缓存条目：查询结果 + 写入时刻（TTL 判定基准，测试可注入）。
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub value: CachedLookup,
    pub created_at: Instant,
}
