//! 词典源注册表：按 id 构造无状态适配器实例。

pub mod baidu;
pub mod bing;
pub mod eudic;
pub mod freedict;
pub mod iciba;
pub mod urban;
pub mod wiktionary;
pub mod youdao;

use std::sync::Arc;

use super::provider::DictProvider;
use super::provider_id::DictProviderId;

/// 按 id 构造源实例（每个源都是无状态单例）。
pub fn build(id: DictProviderId) -> Arc<dyn DictProvider> {
    match id {
        DictProviderId::Youdao => Arc::new(youdao::YoudaoProvider),
        DictProviderId::Baidu => Arc::new(baidu::BaiduProvider),
        DictProviderId::Eudic => Arc::new(eudic::EudicProvider),
        DictProviderId::Bing => Arc::new(bing::BingProvider),
        DictProviderId::Iciba => Arc::new(iciba::IcibaProvider),
        DictProviderId::FreeDict => Arc::new(freedict::FreeDictProvider),
        DictProviderId::Wiktionary => Arc::new(wiktionary::WiktionaryProvider),
        DictProviderId::Urban => Arc::new(urban::UrbanProvider),
    }
}
