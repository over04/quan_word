use scraper::ElementRef;
use serde_json::Value;

use crate::text::element_text;

/// 求值过程中的节点/字符串：JSON 节点、HTML 元素或已生成的文本。
#[derive(Clone)]
pub enum ValueRef<'a> {
    Json(&'a Value),
    Html(ElementRef<'a>),
    Text(String),
}

impl<'a> ValueRef<'a> {
    pub fn as_string(&self) -> String {
        match self {
            Self::Json(Value::String(text)) => text.clone(),
            Self::Json(Value::Number(number)) => number.to_string(),
            Self::Json(Value::Bool(flag)) => flag.to_string(),
            Self::Json(_) => String::new(),
            Self::Html(element) => element_text(*element),
            Self::Text(text) => text.clone(),
        }
    }

    pub fn as_json(&self) -> Option<&'a Value> {
        match self {
            Self::Json(value) => Some(value),
            _ => None,
        }
    }
}
