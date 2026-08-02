use axum::{
    Json,
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use serde_json::json;

use super::asset::Asset;

/// SPA 静态托管：
/// - `/api/*` 未命中路由 → 404 JSON（不回落到 index.html）
/// - 命中嵌入文件 → 按扩展名返回 Content-Type
/// - 其余路径 → 回退 index.html（SPA 兜底）
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api/") {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": "接口不存在" }))).into_response();
    }
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match Asset::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                Body::from(file.data),
            )
                .into_response()
        }
        None => match Asset::get("index.html") {
            Some(file) => (
                [(header::CONTENT_TYPE, "text/html")],
                Body::from(file.data),
            )
                .into_response(),
            None => (StatusCode::NOT_FOUND, "not found").into_response(),
        },
    }
}
