use serde::Deserialize;

use super::extractor::Extractor;
use super::transform::Transform;

/// 命名中间值：按声明顺序求值，后面的变量可引用前面的变量。
#[derive(Debug, Clone, Deserialize)]
pub struct VarSpec {
    pub name: String,
    #[serde(flatten)]
    pub extractor: Extractor,
    #[serde(default)]
    pub transform: Vec<Transform>,
}
