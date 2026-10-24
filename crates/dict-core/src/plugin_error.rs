/// 插件包装载/校验错误（消息直接面向用户与日志）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginError {
    #[error("插件包不是合法 UTF-8")]
    NotUtf8,
    #[error("插件包过大（上限 {max} 字节）")]
    TooLarge { max: usize },
    #[error("插件包 JSON 解析失败: {0}")]
    Json(String),
    #[error("插件 id 非法: {0}")]
    InvalidId(String),
    #[error("插件 api_version 不支持: {0}")]
    UnsupportedApiVersion(u32),
    #[error("插件引擎暂不支持: {0}")]
    UnsupportedEngine(String),
    #[error("插件能力声明无效: {0}")]
    InvalidCapability(String),
    #[error("插件 hosts 配置无效: {0}")]
    InvalidHost(String),
    #[error("插件 URL 非法: {0}")]
    InvalidUrl(String),
    #[error("插件 URL host 未在 hosts 中声明: {0}")]
    HostNotAllowed(String),
    #[error("插件选择器非法: {0}")]
    InvalidSelector(String),
    #[error("插件 JSON 路径非法: {0}")]
    InvalidPath(String),
    #[error("插件配方无效: {0}")]
    InvalidRecipe(String),
    #[error("插件 id 重复: {0}")]
    DuplicateId(String),
    #[error("插件缺少配方段: {0}")]
    MissingRecipe(String),
}
