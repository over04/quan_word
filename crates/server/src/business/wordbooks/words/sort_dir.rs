use std::str::FromStr;

use strum::EnumString;

use super::error::WordError;

/// 列表查询排序方向。
///
/// 查询字符串与变体的映射由 `strum::EnumString` 声明（snake_case）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

impl SortDir {
    pub fn parse(s: &str) -> Result<Self, WordError> {
        Self::from_str(s).map_err(|_| WordError::InvalidSortDir { dir: s.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::SortDir;
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn parses_directions() {
        assert!(matches!(SortDir::parse("asc"), Ok(SortDir::Asc)));
        assert!(matches!(SortDir::parse("desc"), Ok(SortDir::Desc)));
    }

    #[test]
    fn rejects_unknown_direction() {
        assert!(matches!(
            SortDir::parse("sideways"),
            Err(WordError::InvalidSortDir { dir }) if dir == "sideways"
        ));
    }
}
