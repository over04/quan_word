/// 可识别的 POS 缩写 → 白名单书写形式。
#[derive(Debug, Clone, Copy)]
pub struct PosToken {
    pub token: &'static str,
    pub form: &'static str,
}

impl PosToken {
    pub const fn new(token: &'static str, form: &'static str) -> Self {
        Self { token, form }
    }
}
