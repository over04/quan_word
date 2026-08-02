use serde::{Deserialize, Serialize};

/// 一条释义：词性 + 释义内容（领域模型）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub pos: String,
    pub meaning: String,
}
