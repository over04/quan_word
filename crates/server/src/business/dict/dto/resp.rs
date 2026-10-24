use serde::Serialize;
use ts_rs::TS;

use crate::business::dict::entry::DictEntry;
use crate::business::dict::provider_id::DictProviderId;

/// 词典源信息（含是否在配置中启用）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictProviderResp.ts")]
pub struct DictProviderResp {
    pub id: DictProviderId,
    pub label: String,
    pub enabled: bool,
}

/// 单个词典源的失败信息（聚合进抓词响应，不影响其它源）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictProviderErrorResp.ts")]
pub struct DictProviderErrorResp {
    pub provider: DictProviderId,
    pub label: String,
    pub message: String,
}

/// 抓词聚合响应：成功条目 + 失败源列表（单源失败不影响整体，仍为 200）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "DictLookupResp.ts")]
pub struct DictLookupResp {
    pub word: String,
    pub entries: Vec<DictEntry>,
    pub errors: Vec<DictProviderErrorResp>,
}
