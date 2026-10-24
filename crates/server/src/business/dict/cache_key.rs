use dict_core::provider_id::ProviderId;

/// 缓存 / 单飞键：词典源 + 插件版本 + 小写规范化后的单词。
///
/// 插件重载后版本变化会自然失效旧缓存；同 id 覆盖安装也不会读到旧解析结果。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictKey {
    pub provider: ProviderId,
    pub version: String,
    pub word: String,
}

impl DictKey {
    /// 构造键：单词去空白后统一转小写，保证 ` Abandon ` 与 `abandon` 命中同一条缓存。
    pub fn new(provider: ProviderId, version: &str, word: &str) -> Self {
        Self {
            provider,
            version: version.to_string(),
            word: word.trim().to_lowercase(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DictKey;
    use dict_core::provider_id::ProviderId;

    fn id(raw: &str) -> ProviderId {
        ProviderId::new(raw).unwrap()
    }

    #[test]
    fn normalizes_word_case() {
        assert_eq!(
            DictKey::new(id("youdao"), "1.0.0", " Abandon "),
            DictKey::new(id("youdao"), "1.0.0", "abandon")
        );
        assert_ne!(
            DictKey::new(id("youdao"), "1.0.0", "abandon"),
            DictKey::new(id("baidu"), "1.0.0", "abandon")
        );
        assert_ne!(
            DictKey::new(id("youdao"), "1.0.0", "abandon"),
            DictKey::new(id("youdao"), "1.0.1", "abandon")
        );
    }
}
