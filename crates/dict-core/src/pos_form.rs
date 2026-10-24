use lexicon::word_pos::WordPos;

/// 词性枚举 → 白名单书写形式。
#[derive(Debug, Clone, Copy)]
pub struct PosForm {
    pub pos: WordPos,
    pub form: &'static str,
}

impl PosForm {
    pub const fn new(pos: WordPos, form: &'static str) -> Self {
        Self { pos, form }
    }
}
