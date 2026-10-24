use async_trait::async_trait;

use super::entry::DictEntry;
use super::http::DictHttp;
use super::provider_error::DictProviderError;
use super::provider_id::DictProviderId;

/// 词典源适配器：每个上游站点一个无状态实现，只负责「请求 + 解析」。
///
/// 超时、缓存、单飞、熔断、并发限流由 service 统一负责，实现里不做重试。
#[allow(clippy::double_must_use)] // async_trait 展开的 #[must_use] 与 Future 自带重复
#[async_trait]
pub trait DictProvider: Send + Sync {
    /// 源标识（与配置项、查询参数、响应字段一致）。
    fn id(&self) -> DictProviderId;

    /// 中文展示名。
    fn label(&self) -> &'static str;

    /// 抓取一个词条；`NotFound` 表示上游正常响应但未收录该词。
    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError>;
}
