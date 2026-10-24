/// 文本中识别到的一个 POS 标记（字节区间 + 规范书写形式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PosMark {
    pub start: usize,
    pub end: usize,
    pub form: &'static str,
}
