//! 声明式配方：请求、提取、变换、释义策略与建议映射。

pub mod condition;
pub mod contains_spec;
pub mod dedup_mode;
pub mod definition_strategy;
pub mod document;
pub mod empty_behavior;
pub mod execute;
pub mod extractor;
pub mod items_spec;
pub mod json_path;
pub mod lookup_recipe;
pub mod prefer_equals;
pub mod prefer_fallback;
pub mod prefer_spec;
pub mod request_spec;
pub mod suggest_recipe;
pub mod transform;
pub mod unit_transform;
pub mod validate;
pub mod value_ref;
pub mod var_spec;

use serde::Deserialize;

use self::lookup_recipe::LookupRecipe;
use self::suggest_recipe::SuggestRecipe;

/// 一个插件包内的全部配方。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    #[serde(default)]
    pub lookup: Option<LookupRecipe>,
    #[serde(default)]
    pub suggest: Option<SuggestRecipe>,
}
