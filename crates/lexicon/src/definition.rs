use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 一条释义：词性 + 释义内容。
///
/// 对应 `word.definitions` JSON 列的数组元素；同时是请求/响应 DTO 的共享字段类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "Definition.ts")]
pub struct Definition {
    /// 词性（如 "n."、"v."）；空字符串表示未标注
    pub pos: String,
    pub meaning: String,
}
