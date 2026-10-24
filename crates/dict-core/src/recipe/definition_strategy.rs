use serde::Deserialize;

use super::condition::Condition;
use super::extractor::Extractor;
use super::transform::Transform;

/// 释义生成策略：按顺序求值，第一个非空生效。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DefinitionStrategy {
    PosText {
        from: Extractor,
        #[serde(default)]
        separators: Vec<String>,
        #[serde(default)]
        when: Option<Condition>,
        #[serde(default)]
        transform: Vec<Transform>,
    },
    PlainText {
        from: Extractor,
        #[serde(default)]
        separators: Vec<String>,
        #[serde(default)]
        when: Option<Condition>,
        #[serde(default)]
        transform: Vec<Transform>,
    },
    Items {
        from: Extractor,
        #[serde(default)]
        pos: Option<Extractor>,
        #[serde(default)]
        pos_transform: Vec<Transform>,
        meaning: Extractor,
        #[serde(default)]
        split: Vec<String>,
        #[serde(default)]
        when: Option<Condition>,
        #[serde(default)]
        transform: Vec<Transform>,
    },
    Groups {
        from: Extractor,
        #[serde(default)]
        pos: Option<Extractor>,
        #[serde(default)]
        pos_transform: Vec<Transform>,
        items: Extractor,
        #[serde(default)]
        when: Option<Condition>,
        #[serde(default)]
        transform: Vec<Transform>,
    },
}
