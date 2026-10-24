/// 宿主传输层返回的响应（UTF-8 文本）。
#[derive(Debug, Clone)]
pub struct SearchResponse {
    pub status: u16,
    pub body: String,
    pub final_url: String,
}
