use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 批量删除单词请求体。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "BatchDeleteWordsReq.ts")]
pub struct BatchDeleteWordsReq {
    pub ids: Vec<i32>,
}

/// 批量删除单词响应体。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "BatchDeleteWordsResp.ts")]
pub struct BatchDeleteWordsResp {
    #[ts(type = "number")]
    pub deleted: u64,
}
