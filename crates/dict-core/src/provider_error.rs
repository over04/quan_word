/// 单个词典源的抓取错误。
///
/// 不直接映射 HTTP：service 把每个源的失败聚合成 `DictProviderErrorResp`，
/// 只有输入/配置类问题才走 `DictError` → 400。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictProviderError {
    #[error("网络请求失败: {0}")]
    Http(String),
    #[error("请求超时")]
    Timeout,
    #[error("未找到该单词的释义")]
    NotFound,
    #[error("响应解析失败: {0}")]
    Parse(String),
    #[error("响应内容过大")]
    TooLarge,
    #[error("词典源暂时不可用，请稍后再试")]
    Unavailable,
}

impl DictProviderError {
    /// 面向用户的简短中文消息（聚合响应使用，不暴露上游内部细节）。
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Http(_) => "网络请求失败",
            Self::Timeout => "请求超时",
            Self::NotFound => "未找到该单词的释义",
            Self::Parse(_) => "词典数据解析失败",
            Self::TooLarge => "响应内容过大",
            Self::Unavailable => "词典源暂时不可用，请稍后再试",
        }
    }
}
