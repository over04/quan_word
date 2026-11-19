use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use dict_core::suggestion::DictSuggestion;

use super::suggest_cache_entry::SuggestCacheEntry;

type SuggestEntries = HashMap<String, SuggestCacheEntry>;

/// 模糊建议缓存：只缓存非空结果；配置变更后旧请求不能重新填充缓存。
pub struct SuggestCache {
    inner: Mutex<(u64, SuggestEntries)>,
    ttl: Duration,
    cap: usize,
}

impl SuggestCache {
    pub fn new(ttl_secs: u64, cap: usize) -> Self {
        Self {
            inner: Mutex::new((0, HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
            cap: cap.max(1),
        }
    }

    pub fn generation(&self) -> u64 {
        self.inner.lock().0
    }

    pub fn get(&self, generation: u64, word: &str, now: Instant) -> Option<Vec<DictSuggestion>> {
        let guard = self.inner.lock();
        if generation != guard.0 {
            return None;
        }
        let entry = guard.1.get(word)?;
        if now.saturating_duration_since(entry.created_at) >= self.ttl {
            return None;
        }
        Some(entry.suggestions.clone())
    }

    pub fn insert(
        &self,
        generation: u64,
        word: String,
        suggestions: Vec<DictSuggestion>,
        now: Instant,
    ) {
        let (current, guard) = &mut *self.inner.lock();
        if generation != *current || suggestions.is_empty() {
            return;
        }
        if guard.len() >= self.cap && !guard.contains_key(&word) {
            guard.retain(|_, entry| now.saturating_duration_since(entry.created_at) < self.ttl);
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
            word,
            SuggestCacheEntry {
                suggestions,
                created_at: now,
            },
        );
    }

    /// 启停或重载插件后失效，包括变更前发出的请求。
    pub fn clear(&self) {
        let (generation, entries) = &mut *self.inner.lock();
        *generation = generation.wrapping_add(1);
        entries.clear();
    }

    pub fn cleanup(&self, now: Instant) {
        self.inner
            .lock()
            .1
            .retain(|_, entry| now.saturating_duration_since(entry.created_at) < self.ttl);
    }
}
