use serde::Serialize;
use ts_rs::TS;

/// 词典例句：原文 + 可选译文。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictExample.ts")]
pub struct DictExample {
    pub text: String,
    pub translation: Option<String>,
}
