//! 项目统一的路径参数提取器：解析失败返回 `ApiError`（400 中文消息）。
//!
//! 同 `ApiJson`：axum 0.8 不会自动转换 `PathRejection`，本提取器在提取阶段
//! 显式转为 `ApiError`。

use std::ops::Deref;

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::common::error::ApiError;

/// 路径参数包装器，行为同 `axum::extract::Path`，但拒绝类型为 `ApiError`。
pub struct ApiPath<T>(pub T);

impl<T> Deref for ApiPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(ApiError::invalid_path)?;
        Ok(Self(value))
    }
}
