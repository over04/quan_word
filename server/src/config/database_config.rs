/// 数据库配置：URL scheme 决定驱动（sqlite:// 或 postgres://）。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://./data/quan_word.db?mode=rwc".into(),
        }
    }
}
