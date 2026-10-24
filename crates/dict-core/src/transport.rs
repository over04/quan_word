use async_trait::async_trait;

use crate::search_request::SearchRequest;
use crate::search_response::SearchResponse;
use crate::transport_error::TransportError;

/// 宿主网络实现：服务器用 reqwest，移动端可用平台 HTTP，测试用 mock。
#[allow(clippy::double_must_use)] // async_trait 展开的 #[must_use] 与 Future 自带重复
#[async_trait]
pub trait SearchTransport: Send + Sync {
    async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError>;
}
