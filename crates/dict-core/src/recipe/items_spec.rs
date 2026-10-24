use std::collections::BTreeMap;

use serde::Deserialize;

use super::extractor::Extractor;
use super::transform::Transform;

/// 节点列表映射（例句 / 建议共用）。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemsSpec {
    pub from: Extractor,
    #[serde(default)]
    pub fields: BTreeMap<String, Extractor>,
    #[serde(default)]
    pub take: Option<usize>,
    #[serde(default)]
    pub transform: Vec<Transform>,
}
