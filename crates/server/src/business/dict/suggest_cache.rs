use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use dict_core::suggestion::DictSuggestion;

use super::suggest_cache_entry::SuggestCacheEntry;

/// 模糊建议缓存：按规范化单词缓存，TTL 与容量复用词典配置。
pub struct SuggestCache {
    inner: Mutex<HashMap<String, SuggestCacheEntry>>,
    ttl: Duration,
    cap: usize,
}

impl SuggestCache {
    pub fn new(ttl_secs: u64, cap: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: Duration::from_secs(ttl_secs),
            cap: cap.max(1),
        }
    }

    pub fn get(&self, word: &str, now: Instant) -> Option<Vec<DictSuggestion>> {
        let guard = self.inner.lock();
        let entry = guard.get(word)?;
        if now.saturating_duration_since(entry.created_at) >= self.ttl {
            return None;
        }
        Some(entry.suggestions.clone())
    }

    pub fn insert(&self, word: String, suggestions: Vec<DictSuggestion>, now: Instant) {
        let mut guard = self.inner.lock();
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

    /// 清空全部条目（插件重载后建议结果可能变化）。
    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    pub fn cleanup(&self, now: Instant) {
        self.inner
            .lock()
            .retain(|_, entry| now.saturating_duration_since(entry.created_at) < self.ttl);
    }
}
