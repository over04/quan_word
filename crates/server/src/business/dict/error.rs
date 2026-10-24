use crate::common::error::ApiError;

/// 词典域错误：全部是调用方可纠正的输入/配置问题（映射 400）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DictError {
    #[error("请输入要查询的单词")]
    EmptyWord,
    #[error("单词过长（最多 {max} 个字符）")]
    WordTooLong { max: usize },
    #[error("单词只能包含字母、数字、空格、连字符、撇号和句点")]
    InvalidWord,
    #[error("未知的词典源: {provider}")]
    UnknownProvider { provider: String },
    #[error("插件不存在: {id}")]
    PluginNotFound { id: String },
    #[error("词典源未启用: {provider}")]
    ProviderDisabled { provider: String },
    #[error("词典功能未启用")]
    DictDisabled,
    #[error("没有可用的词典源")]
    NoProviders,
    #[error("插件包无效: {0}")]
    InvalidPlugin(String),
    #[error("插件文件操作失败: {0}")]
    PluginIo(String),
}

impl From<DictError> for ApiError {
    fn from(e: DictError) -> Self {
        match e {
            DictError::PluginIo(message) => ApiError::Internal(message),
            DictError::PluginNotFound { id } => ApiError::NotFound(format!("插件不存在: {id}")),
            other => ApiError::BadRequest(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::DictError;
    use crate::common::error::ApiError;

    #[test]
    fn maps_to_bad_request_with_chinese_message() {
        let cases = [
            DictError::EmptyWord,
            DictError::WordTooLong { max: 64 },
            DictError::InvalidWord,
            DictError::UnknownProvider {
                provider: "foo".into(),
            },
            DictError::ProviderDisabled {
                provider: "urban".into(),
            },
            DictError::DictDisabled,
            DictError::NoProviders,
        ];
        for err in cases {
            let msg = err.to_string();
            assert!(!msg.is_empty());
            let api: ApiError = err.into();
            let resp = api.into_response();
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[test]
    fn maps_plugin_not_found_to_404() {
        let api: ApiError = DictError::PluginNotFound { id: "foo".into() }.into();
        assert_eq!(api.into_response().status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn messages_are_user_facing_chinese() {
        assert_eq!(DictError::EmptyWord.to_string(), "请输入要查询的单词");
        assert_eq!(
            DictError::WordTooLong { max: 8 }.to_string(),
            "单词过长（最多 8 个字符）"
        );
        assert!(DictError::UnknownProvider {
            provider: "foo".into()
        }
        .to_string()
        .contains("未知的词典源"));
    }
}
