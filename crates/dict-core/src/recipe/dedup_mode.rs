use serde::Deserialize;

/// 建议去重方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DedupMode {
    WordLowercase,
}
