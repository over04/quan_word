use super::provider_id::DictProviderId;

/// 缓存 / 单飞键：词典源 + 小写规范化后的单词。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictKey {
    pub provider: DictProviderId,
    pub word: String,
}

impl DictKey {
    /// 构造键：单词去空白后统一转小写，保证 ` Abandon ` 与 `abandon` 命中同一条缓存。
    pub fn new(provider: DictProviderId, word: &str) -> Self {
        Self {
            provider,
            word: word.trim().to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DictKey;
    use crate::business::dict::provider_id::DictProviderId;

    #[test]
    fn normalizes_word_case() {
        assert_eq!(
            DictKey::new(DictProviderId::Youdao, " Abandon "),
            DictKey::new(DictProviderId::Youdao, "abandon")
        );
        assert_ne!(
            DictKey::new(DictProviderId::Youdao, "abandon"),
            DictKey::new(DictProviderId::Baidu, "abandon")
        );
    }
}
