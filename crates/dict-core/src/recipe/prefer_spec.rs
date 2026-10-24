use serde::Deserialize;

use super::extractor::Extractor;
use super::prefer_equals::PreferEquals;
use super::prefer_fallback::PreferFallback;

/// 在节点列表里优先选中「某字段等于查询词」的节点。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreferSpec {
    pub from: Box<Extractor>,
    pub field: String,
    #[serde(default)]
    pub equals: PreferEquals,
    #[serde(default)]
    pub ignore_case: bool,
    #[serde(default)]
    pub fallback: PreferFallback,
}
