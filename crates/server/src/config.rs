//! 应用配置：YAML 加载与启动期校验。

mod database;
mod import;
mod server;

use anyhow::{Context, Result};

use self::database::DatabaseConfig;
use self::import::ImportConfig;
use self::server::ServerConfig;

/// 应用配置：server 监听地址 + 数据库连接 + 批量导入。
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    /// 批量导入：解析上限与预览会话缓存生命周期
    #[serde(default)]
    pub import: ImportConfig,
}

impl Config {
    /// 从 YAML 加载配置。路径取环境变量 `QUAN_WORD_CONFIG`，缺省 `./config.yaml`
    /// （模板见仓库根 `config.example.yaml`，已 git 忽略）。
    /// 文件不存在时使用内建默认（sqlite + 0.0.0.0:3000）。
    pub fn load() -> Result<Self> {
        let path =
            std::env::var("QUAN_WORD_CONFIG").unwrap_or_else(|_| "./config.yaml".to_string());
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let cfg: Config = serde_yml::from_str(&content)
                    .with_context(|| format!("解析配置失败: {path}"))?;
                if cfg.import.cache_ttl_secs == 0 || cfg.import.cache_cleanup_secs == 0 {
                    anyhow::bail!(
                        "配置错误: import.cache_ttl_secs 与 import.cache_cleanup_secs 必须大于 0"
                    );
                }
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e).with_context(|| format!("读取配置失败: {path}")),
        }
    }
}
