use crate::common::error::ApiError;

/// 解析分页查询参数：page 默认 1（0 拒绝）；page_size 默认 20，钳制 1..=100。
pub fn parse_paging(page: Option<&str>, page_size: Option<&str>) -> Result<(u64, u64), ApiError> {
    let page = match page {
        None => 1,
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| ApiError::BadRequest("page 必须为正整数".into()))?,
    };
    if page == 0 {
        return Err(ApiError::BadRequest("page 必须为正整数".into()));
    }
    let page_size = match page_size {
        None => 20,
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| ApiError::BadRequest("page_size 必须为正整数".into()))?,
    };
    Ok((page, page_size.clamp(1, 100)))
}

#[cfg(test)]
mod tests {
    use super::parse_paging;

    #[test]
    fn applies_defaults() {
        assert_eq!(parse_paging(None, None).unwrap(), (1, 20));
        assert_eq!(parse_paging(Some("3"), None).unwrap(), (3, 20));
        assert_eq!(parse_paging(None, Some("50")).unwrap(), (1, 50));
    }

    #[test]
    fn rejects_zero_or_non_numeric_page() {
        assert!(parse_paging(Some("0"), None).is_err());
        assert!(parse_paging(Some("abc"), None).is_err());
        assert!(parse_paging(None, Some("abc")).is_err());
    }

    #[test]
    fn clamps_page_size() {
        assert_eq!(parse_paging(None, Some("0")).unwrap(), (1, 1));
        assert_eq!(parse_paging(None, Some("999")).unwrap(), (1, 100));
    }
}
