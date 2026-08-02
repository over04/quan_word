use anyhow::{Context, Result};

use super::database_config::DatabaseConfig;
use super::server_config::ServerConfig;

/// 应用配置：server 监听地址 + 数据库连接。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

impl Config {
    /// 从 YAML 加载配置。路径取环境变量 `QUAN_WORD_CONFIG`，缺省 `./config.yaml`。
    /// 文件不存在时使用内建默认（sqlite + 0.0.0.0:3000）。
    pub fn load() -> Result<Self> {
        let path =
            std::env::var("QUAN_WORD_CONFIG").unwrap_or_else(|_| "./config.yaml".to_string());
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_yml::from_str(&content)
                .with_context(|| format!("解析配置失败: {path}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config {
                server: ServerConfig::default(),
                database: DatabaseConfig::default(),
            }),
            Err(e) => Err(e).with_context(|| format!("读取配置失败: {path}")),
        }
    }
}
