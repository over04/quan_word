use serde::Deserialize;

use super::dedup_mode::DedupMode;
use super::items_spec::ItemsSpec;
use super::request_spec::RequestSpec;

/// 模糊建议配方：请求 + 节点列表映射。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuggestRecipe {
    pub request: RequestSpec,
    pub items: ItemsSpec,
    #[serde(default)]
    pub take: Option<usize>,
    #[serde(default)]
    pub dedup: Option<DedupMode>,
}
