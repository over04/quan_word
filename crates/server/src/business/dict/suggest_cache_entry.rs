use std::time::Instant;

use dict_core::suggestion::DictSuggestion;

/// 模糊建议缓存条目。
#[derive(Debug, Clone)]
pub struct SuggestCacheEntry {
    pub suggestions: Vec<DictSuggestion>,
    pub created_at: Instant,
}
