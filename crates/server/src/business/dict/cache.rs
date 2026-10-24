use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::cache_entry::CacheEntry;
use super::cache_key::DictKey;
use super::cached_lookup::CachedLookup;

/// 词典抓词缓存：正向条目按 `ttl` 过期，负向（未收录）按 `negative_ttl` 过期。
///
/// 容量上限 `cap`：写入时若已满，先清过期条目，再淘汰最旧的一条。
/// 锁为 `parking_lot::Mutex`，全部操作同步且不在 `.await` 期间持有。
pub struct DictCache {
    inner: Mutex<HashMap<DictKey, CacheEntry>>,
    ttl: Duration,
    negative_ttl: Duration,
    cap: usize,
}

impl DictCache {
    pub fn new(ttl_secs: u64, negative_ttl_secs: u64, cap: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
            negative_ttl: Duration::from_secs(negative_ttl_secs),
            cap: cap.max(1),
        }
    }

    /// 读取未过期条目（`now` 由调用方注入，便于测试）。
    pub fn get(&self, key: &DictKey, now: Instant) -> Option<CachedLookup> {
        let guard = self.inner.lock();
        let entry = guard.get(key)?;
        if self.expired(entry, now) {
            return None;
        }
        Some(entry.value.clone())
    }

    /// 写入条目（容量满时先清过期、再淘汰最旧）。
    pub fn insert(&self, key: DictKey, value: CachedLookup, now: Instant) {
        let mut guard = self.inner.lock();
        if guard.len() >= self.cap && !guard.contains_key(&key) {
            guard.retain(|_, entry| !self.expired(entry, now));
            if guard.len() >= self.cap {
                let oldest = guard
                    .iter()
                    .min_by_key(|(_, entry)| entry.created_at)
                    .map(|(key, _)| key.clone());
                if let Some(oldest) = oldest {
                    guard.remove(&oldest);
                }
            }
        }
        guard.insert(
            key,
            CacheEntry {
                value,
                created_at: now,
            },
        );
    }

    /// 清理全部过期条目（后台任务按 `cache_cleanup_secs` 调用）。
    pub fn cleanup(&self, now: Instant) {
        self.inner
            .lock()
            .retain(|_, entry| !self.expired(entry, now));
    }

    /// 当前条目数（含未清理的过期条目）。
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn expired(&self, entry: &CacheEntry, now: Instant) -> bool {
        let ttl = match entry.value {
            CachedLookup::Found(_) => self.ttl,
            CachedLookup::NotFound => self.negative_ttl,
        };
        now.saturating_duration_since(entry.created_at) >= ttl
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::DictCache;
    use crate::business::dict::cache_key::DictKey;
    use crate::business::dict::cached_lookup::CachedLookup;
    use crate::business::dict::entry::DictEntry;
    use crate::business::dict::provider_id::DictProviderId;

    fn entry(word: &str) -> DictEntry {
        DictEntry {
            provider: DictProviderId::Youdao,
            label: "有道词典".into(),
            word: word.into(),
            phonetic: None,
            definitions: Vec::new(),
            examples: Vec::new(),
            audio_url: None,
            source_url: None,
        }
    }

    fn key(word: &str) -> DictKey {
        DictKey::new(DictProviderId::Youdao, word)
    }

    #[test]
    fn caches_positive_hits_until_ttl() {
        let cache = DictCache::new(60, 5, 8);
        let now = Instant::now();
        cache.insert(
            key("abandon"),
            CachedLookup::Found(Arc::new(entry("abandon"))),
            now,
        );
        assert!(matches!(
            cache.get(&key("abandon"), now),
            Some(CachedLookup::Found(_))
        ));
        let later = now + Duration::from_secs(61);
        assert!(
            cache.get(&key("abandon"), later).is_none(),
            "超过 ttl 应失效"
        );
    }

    #[test]
    fn caches_negative_hits_with_shorter_ttl() {
        let cache = DictCache::new(600, 5, 8);
        let now = Instant::now();
        cache.insert(key("zzz"), CachedLookup::NotFound, now);
        assert!(matches!(
            cache.get(&key("zzz"), now + Duration::from_secs(4)),
            Some(CachedLookup::NotFound)
        ));
        assert!(
            cache
                .get(&key("zzz"), now + Duration::from_secs(6))
                .is_none(),
            "负缓存按 negative_ttl 失效"
        );
    }

    #[test]
    fn evicts_oldest_entry_when_full() {
        let cache = DictCache::new(600, 600, 2);
        let now = Instant::now();
        cache.insert(key("a"), CachedLookup::NotFound, now);
        cache.insert(
            key("b"),
            CachedLookup::NotFound,
            now + Duration::from_secs(1),
        );
        assert_eq!(cache.len(), 2);
        cache.insert(
            key("c"),
            CachedLookup::NotFound,
            now + Duration::from_secs(2),
        );
        assert_eq!(cache.len(), 2);
        let check = now + Duration::from_secs(3);
        assert!(cache.get(&key("a"), check).is_none(), "最旧条目应被淘汰");
        assert!(cache.get(&key("b"), check).is_some());
        assert!(cache.get(&key("c"), check).is_some());
    }

    #[test]
    fn cleanup_removes_expired_entries() {
        let cache = DictCache::new(600, 10, 8);
        let now = Instant::now();
        cache.insert(key("a"), CachedLookup::Found(Arc::new(entry("a"))), now);
        cache.insert(
            key("b"),
            CachedLookup::NotFound,
            now + Duration::from_secs(5),
        );
        cache.cleanup(now + Duration::from_secs(61));
        assert_eq!(cache.len(), 1);
        let check = now + Duration::from_secs(61);
        assert!(cache.get(&key("a"), check).is_some());
        assert!(cache.get(&key("b"), check).is_none());
    }

    #[test]
    fn rewriting_same_key_keeps_single_entry() {
        let cache = DictCache::new(60, 60, 1);
        let now = Instant::now();
        cache.insert(key("a"), CachedLookup::NotFound, now);
        cache.insert(
            key("a"),
            CachedLookup::Found(Arc::new(entry("a"))),
            now + Duration::from_secs(1),
        );
        assert_eq!(cache.len(), 1);
        assert!(matches!(
            cache.get(&key("a"), now + Duration::from_secs(2)),
            Some(CachedLookup::Found(_))
        ));
    }
}
