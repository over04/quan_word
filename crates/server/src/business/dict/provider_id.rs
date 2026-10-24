use serde::{Deserialize, Serialize};
use strum::EnumString;
use ts_rs::TS;

/// 支持的词典源标识。
///
/// serde 与 strum 均按 snake_case 映射，保证 JSON 契约、查询参数解析、
/// 配置项写法三者一致（配置里的源名与前端拿到的 id 完全相同）。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, EnumString, TS,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[ts(export, export_to = "DictProviderId.ts")]
pub enum DictProviderId {
    Youdao,
    Baidu,
    Eudic,
    Bing,
    Iciba,
    /// Free Dictionary（dictionaryapi.dev）；id 为 `free_dict`，同时兼容 `freedict` 写法
    #[strum(serialize = "free_dict", serialize = "freedict")]
    FreeDict,
    Wiktionary,
    Urban,
}

impl DictProviderId {
    /// 全部已知词典源（顺序即源列表与聚合结果的展示顺序）。
    pub const ALL: [Self; 8] = [
        Self::Youdao,
        Self::Baidu,
        Self::Eudic,
        Self::Bing,
        Self::Iciba,
        Self::FreeDict,
        Self::Wiktionary,
        Self::Urban,
    ];

    /// 中文展示名。
    pub fn label(self) -> &'static str {
        match self {
            Self::Youdao => "有道词典",
            Self::Baidu => "百度翻译",
            Self::Eudic => "欧路词典",
            Self::Bing => "必应词典",
            Self::Iciba => "金山词霸",
            Self::FreeDict => "Free Dictionary",
            Self::Wiktionary => "维基词典",
            Self::Urban => "Urban Dictionary",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::DictProviderId;

    #[test]
    fn parses_snake_case_ids() {
        for id in DictProviderId::ALL {
            let raw = serde_json::to_value(id).unwrap();
            let raw = raw.as_str().unwrap().to_string();
            assert_eq!(
                DictProviderId::from_str(&raw).unwrap(),
                id,
                "id 应可回环解析"
            );
        }
        assert_eq!(
            DictProviderId::from_str("free_dict").unwrap(),
            DictProviderId::FreeDict
        );
        assert_eq!(
            DictProviderId::from_str("freedict").unwrap(),
            DictProviderId::FreeDict
        );
        assert!(DictProviderId::from_str("unknown").is_err());
    }

    #[test]
    fn every_provider_has_chinese_label() {
        for id in DictProviderId::ALL {
            assert!(!id.label().is_empty());
        }
        assert_eq!(DictProviderId::Youdao.label(), "有道词典");
        assert_eq!(DictProviderId::FreeDict.label(), "Free Dictionary");
    }
}
