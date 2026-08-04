use axum::{
    extract::rejection::{JsonRejection, PathRejection},
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
/// 所有错误路径（含提取器 rejection）统一走此类型，保证响应格式一致。
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
                // 内部细节只进日志，不暴露给客户端
                tracing::error!("数据库错误: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "服务器内部错误".into())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl ApiError {
    /// axum 0.8 的 handler 提取器失败时直接 `rejection.into_response()`，不会自动调用
    /// `From<JsonRejection>` 转换；自定义提取器（`common::http::json::ApiJson`）
    /// 在提取阶段显式调用本方法，把 JSON 解析失败统一为 400 中文消息。
    pub fn invalid_json(r: JsonRejection) -> Self {
        let detail = r.body_text();
        let msg = match &r {
            JsonRejection::MissingJsonContentType(_) => {
                "请求体必须为 JSON（需 Content-Type: application/json）".into()
            }
            JsonRejection::JsonSyntaxError(_) => format!("请求体 JSON 语法错误: {detail}"),
            JsonRejection::JsonDataError(_) => format!("请求体字段错误: {detail}"),
            _ => format!("请求体格式错误: {detail}"),
        };
        ApiError::BadRequest(msg)
    }

    /// 路径参数解析失败（如 id 非数字）→ 400。
    pub fn invalid_path(r: PathRejection) -> Self {
        match r {
            PathRejection::FailedToDeserializePathParams(e) => {
                ApiError::BadRequest(format!("路径参数格式错误: {}", e.body_text()))
            }
            PathRejection::MissingPathParams(_) => ApiError::BadRequest("路径参数缺失".into()),
            _ => ApiError::BadRequest("路径参数格式错误".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::extract::{FromRequest, Json};
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;

    use super::ApiError;

    #[tokio::test]
    async fn maps_status_codes() {
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
        // Db 错误不泄露内部细节
        let body = axum::body::to_bytes(
            ApiError::Db(sea_orm::DbErr::Custom("机密信息".into()))
                .into_response()
                .into_body(),
            1024,
        )
        .await
        .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "服务器内部错误");
    }

    #[tokio::test]
    async fn invalid_json_maps_to_bad_request() {
        let req = Request::builder()
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from("{invalid json"))
            .unwrap();
        let rejection = Json::<serde_json::Value>::from_request(req, &())
            .await
            .unwrap_err();
        let resp = ApiError::invalid_json(rejection).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("JSON 语法错误"));
    }

    #[tokio::test]
    async fn invalid_json_missing_content_type() {
        let req = Request::builder()
            .uri("/")
            .body(Body::from("{\"a\":1}"))
            .unwrap();
        let rejection = Json::<serde_json::Value>::from_request(req, &())
            .await
            .unwrap_err();
        let resp = ApiError::invalid_json(rejection).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["error"].as_str().unwrap().contains("必须为 JSON"));
    }
}
