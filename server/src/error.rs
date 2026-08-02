use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Db(DbErr),
    Internal(anyhow::Error),
}

impl From<DbErr> for ApiError {
    fn from(e: DbErr) -> Self {
        tracing::error!("数据库错误: {e}");
        ApiError::Db(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Db(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("数据库错误: {e}"),
            ),
            ApiError::Internal(e) => {
                tracing::error!("内部错误: {e:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("内部错误: {e}"))
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
