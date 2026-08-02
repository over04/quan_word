use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 批量给单词打标签请求体（只添加，不清除已有标签）。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "BatchTagWordsReq.ts")]
pub struct BatchTagWordsReq {
    pub word_ids: Vec<i32>,
    pub tag_ids: Vec<i32>,
}

/// 批量打标签响应体。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "BatchTagWordsResp.ts")]
pub struct BatchTagWordsResp {
    #[ts(type = "number")]
    pub tagged: u64,
}
