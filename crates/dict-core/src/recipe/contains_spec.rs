use serde::Deserialize;

/// `contains` 条件：某个变量里出现指定片段。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainsSpec {
    pub var: String,
    pub value: String,
}
