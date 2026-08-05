use std::str::FromStr;

use serde::de::Deserializer;
use serde::Deserialize;
use strum::EnumString;

use super::error::WordError;

/// 标签筛选匹配模式（组内匹配方式 / 组间连接词共用）。
///
/// 组内：And=全部命中 / Or=任一命中 / None=没有任何标签（ids 必须为空）；
/// 组间连接词仅用 And / Or。查询字符串与变体的映射由 `strum::EnumString` 声明（snake_case）；
/// JSON 边界反序列化走 `deserialize_tag_match` / `deserialize_links`（中文错误消息）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, Deserialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TagMatch {
    And,
    Or,
    None,
}

impl TagMatch {
    pub fn parse(s: &str) -> Result<Self, WordError> {
        Self::from_str(s).map_err(|_| WordError::InvalidTagMatch {
            mode: s.to_string(),
        })
    }
}

/// `TagGroup.mode` 字段的反序列化：未知值返回与 `InvalidTagMatch` 一致的中文消息。
pub(crate) fn deserialize_tag_match<'de, D>(de: D) -> Result<TagMatch, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(de)?;
    TagMatch::from_str(&raw).map_err(|_| {
        serde::de::Error::custom(format!(
            "不支持的标签匹配模式: {raw}，可选：and / or / none"
        ))
    })
}

/// `TagGroupsParam.links`（连接词数组）的反序列化：逐项解析，未知值报中文消息。
pub(crate) fn deserialize_links<'de, D>(de: D) -> Result<Vec<TagMatch>, D::Error>
where
    D: Deserializer<'de>,
{
    let raws = Vec::<String>::deserialize(de)?;
    raws.into_iter()
        .map(|raw| {
            TagMatch::from_str(&raw).map_err(|_| {
                serde::de::Error::custom(format!("不支持的连接方式: {raw}，可选：and / or"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::TagMatch;
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn parses_and() {
        assert!(matches!(TagMatch::parse("and"), Ok(TagMatch::And)));
    }

    #[test]
    fn parses_or() {
        assert!(matches!(TagMatch::parse("or"), Ok(TagMatch::Or)));
    }

    #[test]
    fn rejects_unknown_mode() {
        assert!(matches!(
            TagMatch::parse("sideways"),
            Err(WordError::InvalidTagMatch { mode }) if mode == "sideways"
        ));
    }

    #[derive(Debug, Deserialize)]
    struct Wrap {
        #[serde(deserialize_with = "super::deserialize_tag_match")]
        mode: TagMatch,
    }

    #[derive(Debug, Deserialize)]
    struct LinkWrap {
        #[serde(deserialize_with = "super::deserialize_links")]
        links: Vec<TagMatch>,
    }

    #[test]
    fn deserializes_mode_and_links() {
        let w: Wrap = serde_json::from_str(r#"{"mode":"or"}"#).unwrap();
        assert_eq!(w.mode, TagMatch::Or);
        let l: LinkWrap = serde_json::from_str(r#"{"links":["and","or"]}"#).unwrap();
        assert_eq!(l.links, vec![TagMatch::And, TagMatch::Or]);
    }

    #[test]
    fn rejects_bad_mode_with_chinese_message() {
        let err = serde_json::from_str::<Wrap>(r#"{"mode":"sideways"}"#).unwrap_err();
        assert!(err.to_string().contains("不支持的标签匹配模式: sideways"));
    }

    #[test]
    fn rejects_bad_link_with_chinese_message() {
        let err = serde_json::from_str::<LinkWrap>(r#"{"links":["xor"]}"#).unwrap_err();
        assert!(err.to_string().contains("不支持的连接方式: xor"));
    }
}
