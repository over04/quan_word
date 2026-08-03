//! 错误归一中间件：兜底把任何非 JSON 的 4xx/5xx 响应转为 `{"error": msg}`。
//!
//! 业务错误（`ApiError`）、提取器 rejection、SPA 404 均已直接返回 JSON；
//! 本中间件只处理遗漏路径（axum 默认的 405 Method Not Allowed、未捕获
//! 500 纯文本等），保证所有错误响应格式统一。

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// 响应体读取上限（错误消息不会超过此值）。
const MAX_BODY: usize = 64 * 1024;

pub async fn normalize_error(req: Request, next: Next) -> Response {
    let response = next.run(req).await;
    let status = response.status();
    if !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let is_json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.starts_with("application/json"));
    if is_json {
        return response;
    }
    let (_parts, body) = response.into_parts();
    let bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    let msg = match status {
        StatusCode::METHOD_NOT_ALLOWED => "请求方法不允许".to_string(),
        StatusCode::NOT_FOUND if text.is_empty() => "接口不存在".to_string(),
        _ if text.is_empty() => format!("请求失败 (HTTP {status})"),
        _ => text,
    };
    (status, Json(json!({ "error": msg }))).into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::{IntoResponse, Response},
        Router,
    };
    use tower::ServiceExt;

    use super::normalize_error;

    async fn run(req: Request<Body>) -> Response {
        Router::new()
            .route(
                "/plain",
                axum::routing::get(|| async { (StatusCode::BAD_REQUEST, "纯文本错误").into_response() }),
            )
            .route(
                "/method",
                axum::routing::get(|| async {}),
            )
            .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found").into_response() })
            .layer(axum::middleware::from_fn(normalize_error))
            .with_state(())
            .oneshot(req)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn wraps_plain_text_4xx() {
        let resp = run(
            Request::builder()
                .uri("/plain")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "纯文本错误");
    }

    #[tokio::test]
    async fn wraps_method_not_allowed() {
        let resp = run(
            Request::builder()
                .uri("/method")
                .method("POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "请求方法不允许");
    }

    #[tokio::test]
    async fn wraps_fallback_not_found() {
        let resp = run(
            Request::builder()
                .uri("/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "Not Found");
    }

    #[tokio::test]
    async fn leaves_success_responses_untouched() {
        let resp = run(
            Request::builder()
                .uri("/method")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
