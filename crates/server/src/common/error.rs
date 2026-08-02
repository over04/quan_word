use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde_json::json;

/// HTTP 边界聚合错误：把各业务域的领域错误映射为传输错误。
///
/// 领域错误在各自 `business/<域>/error.rs` 定义，此处只负责
/// 状态码与 `{"error": msg}` 响应体（用户可见中文消息）。
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("数据库错误: {0}")]
    Db(#[from] DbErr),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            ApiError::Db(e) => {
                tracing::error!("数据库错误: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("数据库错误: {e}"),
                )
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::ApiError;

    #[test]
    fn maps_status_codes() {
        assert_eq!(
            ApiError::NotFound("x".into()).into_response().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::BadRequest("x".into()).into_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::Unauthorized("x".into()).into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::Db(sea_orm::DbErr::Custom("x".into()))
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
