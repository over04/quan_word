use serde::Deserialize;

/// 无参数变换。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitTransform {
    CleanHtml,
    CollapseWs,
    Trim,
    NormalizePhonetic,
    MapEnglishPos,
    CanonicalPos,
    Lowercase,
    UrlEncode,
}
