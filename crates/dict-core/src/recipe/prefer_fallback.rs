use serde::Deserialize;

/// 没有匹配项时的回退策略。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferFallback {
    #[default]
    First,
    None,
}
