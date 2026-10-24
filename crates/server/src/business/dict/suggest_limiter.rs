use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// 限流窗口：60s。
const WINDOW: Duration = Duration::from_secs(60);
/// 单窗口配额：每 IP 最多 120 次建议请求（≈2 req/s，真人打字远远到不了，刷接口会立刻撞上）。
const QUOTA: u32 = 120;
/// 记录的 IP 上限，超出整体清空（单机应用，不需要精细 LRU）。
const MAX_IPS: usize = 2048;

/// 建议接口的固定窗口限流。
///
/// `/api/dict/suggest` 无鉴权且每次都会扇出到有道；`SUGGEST_LIMIT` 提到 24 条之后
/// 单请求成本更高，随机词又能绕过按词缓存 —— 这里按 IP 兜一层，防止被刷穿上游。
/// 只记 `(窗口起点, 已用次数)` 两个值，锁内无 await。
pub struct SuggestRateLimiter {
    windows: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl SuggestRateLimiter {
    pub fn new() -> Self {
        Self {
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// 记一次请求；超出配额返回 `false`（调用方映射为 429）。
    pub fn allow(&self, ip: IpAddr, now: Instant) -> bool {
        let mut windows = self.windows.lock();
        if windows.len() > MAX_IPS {
            windows.clear();
        }
        let entry = windows.entry(ip).or_insert((now, 0));
        if now.duration_since(entry.0) >= WINDOW {
            *entry = (now, 0);
        }
        if entry.1 >= QUOTA {
            return false;
        }
        entry.1 += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(last: u8) -> IpAddr {
        IpAddr::from([127, 0, 0, last])
    }

    #[test]
    fn allows_up_to_quota_then_blocks() {
        let limiter = SuggestRateLimiter::new();
        let now = Instant::now();
        for _ in 0..QUOTA {
            assert!(limiter.allow(ip(1), now));
        }
        assert!(!limiter.allow(ip(1), now));
        // 其它 IP 不受影响
        assert!(limiter.allow(ip(2), now));
    }

    #[test]
    fn resets_after_window() {
        let limiter = SuggestRateLimiter::new();
        let start = Instant::now();
        for _ in 0..QUOTA {
            assert!(limiter.allow(ip(3), start));
        }
        assert!(!limiter.allow(ip(3), start));
        assert!(limiter.allow(ip(3), start + WINDOW));
    }
}
