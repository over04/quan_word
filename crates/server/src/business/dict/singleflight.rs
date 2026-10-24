use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;

use super::cache_key::DictKey;

/// 单飞闸门表：同一个 key 的并发请求共享一把异步互斥锁，
/// 只有第一个真正请求上游，其余等待后重新查缓存。
#[derive(Default)]
pub struct DictSingleflight {
    inner: Mutex<HashMap<DictKey, Arc<AsyncMutex<()>>>>,
}

impl DictSingleflight {
    pub fn new() -> Self {
        Self::default()
    }

    /// 取得（必要时创建）该 key 的闸门。
    pub fn gate(&self, key: &DictKey) -> Arc<AsyncMutex<()>> {
        let mut guard = self.inner.lock();
        Arc::clone(guard.entry(key.clone()).or_default())
    }

    /// 释放闸门：仅当没有其它等待者（只剩表内一份引用）时移除，避免表无限增长。
    ///
    /// 调用方必须在仍持有 `gate()` 返回的 Arc 时调用本方法。
    pub fn release(&self, key: &DictKey) {
        let mut guard = self.inner.lock();
        let removable = guard
            .get(key)
            .is_some_and(|gate| Arc::strong_count(gate) == 2);
        if removable {
            guard.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dict_core::provider_id::ProviderId;

    use super::DictSingleflight;
    use crate::business::dict::cache_key::DictKey;

    fn key(word: &str) -> DictKey {
        DictKey::new(ProviderId::new("youdao").unwrap(), "1.0.0", word)
    }

    #[test]
    fn shares_gate_per_key_and_releases_when_idle() {
        let flight = DictSingleflight::new();
        let first = flight.gate(&key("abandon"));
        let second = flight.gate(&key("abandon"));
        assert!(Arc::ptr_eq(&first, &second), "同 key 应共享闸门");

        // 仍有等待者：不释放
        flight.release(&key("abandon"));
        let third = flight.gate(&key("abandon"));
        assert!(Arc::ptr_eq(&third, &first));
        drop(third);
        drop(second);

        // 只剩调用方一份引用：释放后表内条目移除
        flight.release(&key("abandon"));
        let fresh = flight.gate(&key("abandon"));
        assert!(!Arc::ptr_eq(&fresh, &first), "空闲后应重建闸门");
    }

    #[test]
    fn different_keys_use_different_gates() {
        let flight = DictSingleflight::new();
        let a = flight.gate(&key("abandon"));
        let b = flight.gate(&key("abandoned"));
        assert!(!Arc::ptr_eq(&a, &b));
    }
}
