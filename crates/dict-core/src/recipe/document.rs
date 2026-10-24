use scraper::Html;
use serde_json::Value;

/// 已解析的响应文档：JSON 或 HTML（按首个非空白字符判定）。
pub enum Document {
    Json(Value),
    Html(Html),
}

impl Document {
    pub fn detect(body: &str) -> Self {
        let trimmed = body.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(value) = serde_json::from_str::<Value>(body) {
                return Self::Json(value);
            }
        }
        Self::Html(Html::parse_document(body))
    }

    pub fn json(&self) -> Option<&Value> {
        match self {
            Self::Json(value) => Some(value),
            Self::Html(_) => None,
        }
    }

    pub fn html(&self) -> Option<&Html> {
        match self {
            Self::Html(html) => Some(html),
            Self::Json(_) => None,
        }
    }
}
