use super::entry::DictEntry;
use super::provider_error::DictProviderError;
use super::provider_id::DictProviderId;

/// 单个词典源的抓取结果：源标识 + 成功词条或错误。
#[derive(Debug)]
pub struct ProviderOutcome {
    pub provider: DictProviderId,
    pub result: Result<DictEntry, DictProviderError>,
}
