//! 应用配置：YAML 加载与启动期校验。

mod database;
pub(crate) mod dict;
mod import;
mod server;

use std::path::Path;

use anyhow::{Context, Result};

use self::database::DatabaseConfig;
use self::dict::DictConfig;
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
    /// 词典抓词：源开关、超时、缓存/熔断参数
    #[serde(default)]
    pub dict: DictConfig,
}

impl Config {
    /// 从 YAML 加载配置。路径取环境变量 `QUAN_WORD_CONFIG`，缺省 `./config.yaml`
    /// （模板见仓库根 `config.example.yaml`，已 git 忽略）。
    /// 只有未指定路径且默认文件不存在时，才使用内建默认。
    pub fn load() -> Result<Self> {
        match std::env::var_os("QUAN_WORD_CONFIG") {
            Some(path) => Self::load_from(Path::new(&path), false),
            None => Self::load_from(Path::new("./config.yaml"), true),
        }
    }

    fn load_from(path: &Path, allow_missing: bool) -> Result<Self> {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if allow_missing && e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e).with_context(|| format!("读取配置失败: {}", path.display())),
        };
        let config: Self = serde_yml::from_str(&content)
            .with_context(|| format!("解析配置失败: {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self
            .server
            .auth_key
            .as_ref()
            .is_some_and(|key| key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_graphic()))
        {
            anyhow::bail!("配置错误: server.auth_key 必须为非空的可见 ASCII，且不含空格");
        }
        if self.import.cache_ttl_secs == 0 || self.import.cache_cleanup_secs == 0 {
            anyhow::bail!(
                "配置错误: import.cache_ttl_secs 与 import.cache_cleanup_secs 必须大于 0"
            );
        }
        if let Err(msg) = self.dict.validate() {
            anyhow::bail!("配置错误: {msg}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn only_missing_default_path_can_use_defaults() {
        let missing =
            std::env::temp_dir().join(format!("quan-word-missing-config-{}", std::process::id()));
        let err = Config::load_from(&missing, false).unwrap_err();
        assert!(err.to_string().contains("读取配置失败"));
        assert!(Config::load_from(&missing, true)
            .unwrap()
            .server
            .auth_key
            .is_none());
    }

    #[test]
    fn rejects_invalid_http_access_keys() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());
        for key in ["", "中文", "with space", "trailing\n"] {
            config.server.auth_key = Some(key.into());
            let err = config.validate().unwrap_err().to_string();
            assert!(err.contains("server.auth_key"));
        }
        config.server.auth_key = Some("test-123_!".into());
        assert!(config.validate().is_ok());
    }
}
