use std::time::Instant;

/// 单个词典源的熔断状态：连续失败次数 + 最近一次跳闸时刻。
#[derive(Debug, Clone, Default)]
pub struct BreakerState {
    pub failures: u32,
    pub opened_at: Option<Instant>,
}
