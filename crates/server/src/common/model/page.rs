use serde::Serialize;
use ts_rs::TS;

/// 分页响应体（API 页码从 1 开始）。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "Page.ts", rename = "Page")]
pub struct PageResp<T> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: u64,
    #[ts(type = "number")]
    pub page: u64,
    #[ts(type = "number")]
    pub page_size: u64,
    #[ts(type = "number")]
    pub total_pages: u64,
}
