use dict_core::entry::DictEntry;
use dict_core::provider_error::DictProviderError;
use dict_core::provider_id::ProviderId;

/// 单个词典源的抓取结果：源标识 + 成功词条或错误。
#[derive(Debug)]
pub struct ProviderOutcome {
    pub provider: ProviderId,
    pub result: Result<DictEntry, DictProviderError>,
}
