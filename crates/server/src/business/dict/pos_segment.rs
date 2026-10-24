/// POS 切分后的片段：词性书写形式（空串 = 无词性前缀）+ 片段原文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PosSegment {
    pub pos: String,
    pub text: String,
}
