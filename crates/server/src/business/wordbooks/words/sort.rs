use std::str::FromStr;

use strum::EnumString;

use super::error::WordError;

/// 列表查询排序字段白名单。
///
/// 查询字符串与变体的映射由 `strum::EnumString` 声明（snake_case），
/// 不在业务层手写字面量匹配。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum SortField {
    Spelling,
    CreatedAt,
    UpdatedAt,
}

impl SortField {
    pub fn parse(s: &str) -> Result<Self, WordError> {
        Self::from_str(s).map_err(|_| WordError::InvalidSortField {
            field: s.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SortField;
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn parses_whitelist_fields() {
        assert!(matches!(
            SortField::parse("spelling"),
            Ok(SortField::Spelling)
        ));
        assert!(matches!(
            SortField::parse("created_at"),
            Ok(SortField::CreatedAt)
        ));
        assert!(matches!(
            SortField::parse("updated_at"),
            Ok(SortField::UpdatedAt)
        ));
    }

    #[test]
    fn rejects_unknown_field() {
        assert!(matches!(
            SortField::parse("name"),
            Err(WordError::InvalidSortField { field }) if field == "name"
        ));
    }
}
