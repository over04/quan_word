use std::str::FromStr;

use serde::de::Deserializer;
use serde::Deserialize;
use strum::EnumString;
use ts_rs::TS;

/// 导入预览行筛选：全部 / 仅有错误 / 仅重复组。
///
/// JSON 值与请求体字符串一致（snake_case）；`deserialize_filter` 在请求体
/// 反序列化边界解析一次，非法值返回中文错误消息，业务层只匹配枚举变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, TS)]
#[strum(serialize_all = "snake_case")]
#[ts(export, export_to = "ImportFilter.ts")]
#[ts(rename_all = "snake_case")]
pub enum ImportFilter {
    #[default]
    All,
    Error,
    Duplicate,
}

/// `ImportRowsReq.filter` 字段的反序列化：未知值给出与旧 `InvalidImportFilter`
/// 一致的中文消息（经 `ApiJson` 的 400 响应透出，不走 serde 英文默认文本）。
pub(crate) fn deserialize_filter<'de, D>(de: D) -> Result<ImportFilter, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(de)?;
    ImportFilter::from_str(&raw).map_err(|_| {
        serde::de::Error::custom(format!(
            "不支持的导入筛选: {raw}，可选：all / error / duplicate"
        ))
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::ImportFilter;

    #[derive(Debug, Deserialize)]
    struct Wrap {
        #[serde(default, deserialize_with = "super::deserialize_filter")]
        filter: ImportFilter,
    }

    #[test]
    fn parses_all_modes() {
        for (raw, expected) in [
            ("all", ImportFilter::All),
            ("error", ImportFilter::Error),
            ("duplicate", ImportFilter::Duplicate),
        ] {
            let wrap: Wrap = serde_json::from_str(&format!(r#"{{"filter":"{raw}"}}"#)).unwrap();
            assert_eq!(wrap.filter, expected);
        }
    }

    #[test]
    fn missing_field_defaults_to_all() {
        let wrap: Wrap = serde_json::from_str(r#"{"token":"t"}"#).unwrap();
        assert_eq!(wrap.filter, ImportFilter::All);
    }

    #[test]
    fn rejects_unknown_mode_with_chinese_message() {
        let err = serde_json::from_str::<Wrap>(r#"{"filter":"sideways"}"#).unwrap_err();
        assert!(err.to_string().contains("不支持的导入筛选: sideways"));
    }
}
