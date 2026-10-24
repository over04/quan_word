use std::sync::Arc;

use dict_core::entry::DictEntry;

/// 缓存里的查询结果：命中词条，或上游明确「未收录」（负缓存）。
#[derive(Debug, Clone)]
pub enum CachedLookup {
    Found(Arc<DictEntry>),
    NotFound,
}
