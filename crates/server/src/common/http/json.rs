//! 项目统一的 JSON 请求体提取器：解析失败返回 `ApiError`（400 中文消息）。
//!
//! axum 0.8 的 handler 提取器失败时直接 `rejection.into_response()`，不会自动
//! 调用 `From<JsonRejection>` 转换；本提取器在提取阶段显式把 `JsonRejection`
//! 转为 `ApiError`，保证所有 handler 的错误类型从提取环节起就统一为 `ApiError`。

use std::ops::Deref;

use axum::extract::{FromRequest, Json, Request};
use serde::de::DeserializeOwned;

use crate::common::error::ApiError;

/// JSON 请求体包装器，行为同 `axum::extract::Json`，但拒绝类型为 `ApiError`。
pub struct ApiJson<T>(pub T);

impl<T> Deref for ApiJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) =
            Json::<T>::from_request(req, state)
                .await
                .map_err(ApiError::invalid_json)?;
        Ok(Self(value))
    }
}
