use super::error::WordError;

/// 单词列表浏览顺序：纸质书浏览模式（id / 字母 / seeded 随机打乱）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordOrder {
    IdAsc,
    IdDesc,
    Spelling,
    Random(String),
}

impl WordOrder {
    /// 解析 order / seed 查询参数；白名单外的值返回错误。
    ///
    /// 未用 `strum::EnumString`：`Random(String)` 携带 seed 数据，
    /// strum 不支持带字段变体；且 random 依赖第二个参数（seed）的跨参数校验。
    pub fn parse(order: Option<&str>, seed: Option<&str>) -> Result<Self, WordError> {
        match order {
            None | Some("id_asc") => Ok(Self::IdAsc),
            Some("id_desc") => Ok(Self::IdDesc),
            Some("spelling") => Ok(Self::Spelling),
            Some("random") => match seed.filter(|s| !s.is_empty()) {
                Some(seed) => Ok(Self::Random(seed.to_string())),
                None => Err(WordError::RandomWithoutSeed),
            },
            Some(other) => Err(WordError::InvalidOrder {
                order: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WordOrder;
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn defaults_to_id_asc() {
        assert!(matches!(WordOrder::parse(None, None), Ok(WordOrder::IdAsc)));
    }

    #[test]
    fn parses_whitelist_orders() {
        assert!(matches!(
            WordOrder::parse(Some("id_asc"), None),
            Ok(WordOrder::IdAsc)
        ));
        assert!(matches!(
            WordOrder::parse(Some("id_desc"), None),
            Ok(WordOrder::IdDesc)
        ));
        assert!(matches!(
            WordOrder::parse(Some("spelling"), None),
            Ok(WordOrder::Spelling)
        ));
        assert!(matches!(
            WordOrder::parse(Some("random"), Some("abc")),
            Ok(WordOrder::Random(seed)) if seed == "abc"
        ));
    }

    #[test]
    fn random_requires_seed() {
        assert!(matches!(
            WordOrder::parse(Some("random"), None),
            Err(WordError::RandomWithoutSeed)
        ));
        assert!(matches!(
            WordOrder::parse(Some("random"), Some("")),
            Err(WordError::RandomWithoutSeed)
        ));
    }

    #[test]
    fn rejects_unknown_order() {
        assert!(matches!(
            WordOrder::parse(Some("by_date"), None),
            Err(WordError::InvalidOrder { order }) if order == "by_date"
        ));
    }
}
