use serde::Deserialize;

use super::suggest_payload::SuggestPayload;

/// 有道 suggest 接口的顶层响应。
#[derive(Debug, Deserialize)]
pub struct SuggestWire {
    pub data: Option<SuggestPayload>,
}
