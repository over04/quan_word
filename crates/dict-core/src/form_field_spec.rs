use serde::Deserialize;

/// 表单字段（值可含 `{word}` 占位符）。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormFieldSpec {
    pub name: String,
    pub value: String,
}
