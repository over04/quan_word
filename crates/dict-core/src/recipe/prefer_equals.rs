use serde::Deserialize;

/// 优先匹配的比较方式（当前只有「等于查询词」）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreferEquals {
    #[default]
    Query,
}
