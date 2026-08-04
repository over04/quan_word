use strum::EnumString;

/// 导入文件类型（按上传文件名扩展名解析）：csv / xlsx / xls / ods。
///
/// 字符串到枚举的映射由 `strum::EnumString` 声明（snake_case），
/// service 边界解析一次；不支持的后缀由 `UnsupportedFormat` 报错。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ImportFileType {
    Csv,
    Xlsx,
    Xls,
    Ods,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::ImportFileType;

    #[test]
    fn parses_supported_extensions() {
        for (raw, expected) in [
            ("csv", ImportFileType::Csv),
            ("xlsx", ImportFileType::Xlsx),
            ("xls", ImportFileType::Xls),
            ("ods", ImportFileType::Ods),
        ] {
            assert!(matches!(
                ImportFileType::from_str(raw),
                Ok(actual) if actual == expected
            ));
        }
    }

    #[test]
    fn rejects_unknown_extension() {
        assert!(ImportFileType::from_str("pdf").is_err());
        assert!(ImportFileType::from_str("").is_err());
        assert!(ImportFileType::from_str("CSV").is_err());
    }
}
