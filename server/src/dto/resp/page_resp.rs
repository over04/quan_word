use serde::Serialize;

/// 分页响应体。
#[derive(Debug, Serialize)]
pub struct PageResp<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub total_pages: u64,
}
