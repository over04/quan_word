use serde::Deserialize;

use super::definition_strategy::DefinitionStrategy;
use super::empty_behavior::EmptyBehavior;
use super::extractor::Extractor;
use super::items_spec::ItemsSpec;
use super::request_spec::RequestSpec;
use super::var_spec::VarSpec;

/// 抓词配方：请求 → 变量 → 字段映射 → 释义策略 → 例句。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupRecipe {
    pub request: RequestSpec,
    #[serde(default)]
    pub vars: Vec<VarSpec>,
    #[serde(default)]
    pub word: Option<Extractor>,
    #[serde(default)]
    pub phonetic: Option<Extractor>,
    #[serde(default)]
    pub audio_url: Option<Extractor>,
    #[serde(default)]
    pub source_url: Option<Extractor>,
    #[serde(default)]
    pub definitions: Vec<DefinitionStrategy>,
    #[serde(default)]
    pub examples: Option<ItemsSpec>,
    #[serde(default)]
    pub empty: EmptyBehavior,
}
