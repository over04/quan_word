use crate::provider_error::DictProviderError;

/// 传输层错误（由宿主实现映射到统一错误）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("网络请求失败: {0}")]
    Http(String),
    #[error("请求超时")]
    Timeout,
    #[error("响应内容过大")]
    TooLarge,
    #[error("未找到该单词的释义")]
    NotFound,
    #[error("重定向到未授权主机: {0}")]
    RedirectBlocked(String),
}

impl TransportError {
    pub fn into_provider_error(self) -> DictProviderError {
        match self {
            Self::Http(message) => DictProviderError::Http(message),
            Self::Timeout => DictProviderError::Timeout,
            Self::TooLarge => DictProviderError::TooLarge,
            Self::NotFound => DictProviderError::NotFound,
            Self::RedirectBlocked(message) => DictProviderError::Http(message),
        }
    }
}
