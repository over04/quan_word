use std::str::FromStr;

use strum::EnumString;

use super::error::WordError;

/// 标签筛选匹配模式。
///
/// 查询字符串与变体的映射由 `strum::EnumString` 声明（snake_case）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TagMatch {
    And,
    Or,
}

impl TagMatch {
    pub fn parse(s: &str) -> Result<Self, WordError> {
        Self::from_str(s).map_err(|_| WordError::InvalidTagMatch { mode: s.to_string() })
    }

    /// 洗牌缓存 key 分量（common 层不依赖 business 类型，key 用字符串表示模式）。
    pub fn cache_code(self) -> &'static str {
        match self {
            TagMatch::And => "and",
            TagMatch::Or => "or",
        }
    }
}

#[cfg(test)]
mod tests {
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
}
