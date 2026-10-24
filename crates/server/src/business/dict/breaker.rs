use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use dict_core::provider_id::ProviderId;

use super::breaker_state::BreakerState;

/// 词典源熔断器：连续失败达阈值即跳闸，冷却期内直接快速失败（不发请求）。
pub struct DictBreakers {
    inner: Mutex<HashMap<ProviderId, BreakerState>>,
    threshold: u32,
    cooldown: Duration,
}

impl DictBreakers {
    pub fn new(threshold: u32, cooldown_secs: u64) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            threshold: threshold.max(1),
            cooldown: Duration::from_secs(cooldown_secs),
        }
    }

    /// 是否允许请求：未跳闸或冷却已结束（半开，允许试探一次）。
    pub fn allow(&self, id: &ProviderId, now: Instant) -> bool {
        let guard = self.inner.lock();
        match guard.get(id).and_then(|state| state.opened_at) {
            Some(opened_at) => now.saturating_duration_since(opened_at) >= self.cooldown,
            None => true,
        }
    }

    /// 请求成功：复位该源状态。
    pub fn on_success(&self, id: &ProviderId) {
        self.inner.lock().remove(id);
    }

    /// 请求失败：累计失败次数，达阈值则跳闸。
    pub fn on_failure(&self, id: &ProviderId, now: Instant) {
        let mut guard = self.inner.lock();
        let state = guard.entry(id.clone()).or_default();
        state.failures = state.failures.saturating_add(1);
        if state.failures >= self.threshold {
            state.opened_at = Some(now);
        }
    }

    /// 清空全部熔断状态（插件重载后旧故障不应继续短路）。
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use dict_core::provider_id::ProviderId;

    use super::DictBreakers;

    fn id(raw: &str) -> ProviderId {
        ProviderId::new(raw).unwrap()
    }

    #[test]
    fn opens_after_threshold_then_recovers_after_cooldown() {
        let breakers = DictBreakers::new(3, 60);
        let now = Instant::now();
        let youdao = id("youdao");
        assert!(breakers.allow(&youdao, now));
        breakers.on_failure(&youdao, now);
        breakers.on_failure(&youdao, now);
        assert!(breakers.allow(&youdao, now), "未达阈值仍放行");
        breakers.on_failure(&youdao, now);
        assert!(
            !breakers.allow(&youdao, now + Duration::from_secs(59)),
            "冷却期内应快速失败"
        );
        assert!(
            breakers.allow(&youdao, now + Duration::from_secs(60)),
            "冷却结束进入半开放行"
        );
    }

    #[test]
    fn success_resets_failure_counter() {
        let breakers = DictBreakers::new(2, 60);
        let now = Instant::now();
        let youdao = id("youdao");
        breakers.on_failure(&youdao, now);
        breakers.on_success(&youdao);
        breakers.on_failure(&youdao, now);
        assert!(breakers.allow(&youdao, now), "成功应清零失败计数");
    }

    #[test]
    fn breakers_are_isolated_per_provider() {
        let breakers = DictBreakers::new(1, 60);
        let now = Instant::now();
        let youdao = id("youdao");
        let baidu = id("baidu");
        breakers.on_failure(&youdao, now);
        assert!(!breakers.allow(&youdao, now));
        assert!(breakers.allow(&baidu, now), "其它源不受影响");
    }

    #[test]
    fn clear_resets_all_states() {
        let breakers = DictBreakers::new(1, 60);
        let now = Instant::now();
        let youdao = id("youdao");
        breakers.on_failure(&youdao, now);
        assert!(!breakers.allow(&youdao, now));
        breakers.clear();
        assert!(breakers.allow(&youdao, now));
    }
}
