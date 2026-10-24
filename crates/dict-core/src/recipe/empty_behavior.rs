use serde::Deserialize;

/// 释义为空时的错误类型（欧路把「页面没解析到释义」视为解析失败）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmptyBehavior {
    #[default]
    NotFound,
    Parse,
}
