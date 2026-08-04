use strum::EnumString;

/// 导入模板下载格式白名单（查询参数 `format`，缺省 csv）。
///
/// 字符串到枚举的映射由 `strum::EnumString` 声明，router 边界解析一次。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TemplateFormat {
    Csv,
    Xlsx,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::TemplateFormat;

    #[test]
    fn parses_whitelist_formats() {
        assert!(matches!(
            TemplateFormat::from_str("csv"),
            Ok(TemplateFormat::Csv)
        ));
        assert!(matches!(
            TemplateFormat::from_str("xlsx"),
            Ok(TemplateFormat::Xlsx)
        ));
    }

    #[test]
    fn rejects_unknown_format() {
        assert!(TemplateFormat::from_str("ods").is_err());
        assert!(TemplateFormat::from_str("").is_err());
    }
}
