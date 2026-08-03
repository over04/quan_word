//! 访问密钥中间件：config `server.auth_key` 配置后，所有 /api 请求必须携带密钥。
//!
//! 支持 `Authorization: Bearer <key>` 与 `X-Api-Key: <key>` 两种携带方式；
//! 未配置密钥时直接放行（对现有部署零影响）。比较使用恒定时间算法。

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::common::error::ApiError;
use crate::common::state::AppState;

/// 请求拦截：校验访问密钥。作为 api 路由的 layer 挂载（静态资源不受保护）。
pub async fn require_api_key(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if let Some(expected) = state.api_key.as_deref() {
        let got = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .or_else(|| req.headers().get("x-api-key").and_then(|v| v.to_str().ok()));
        let ok = got.is_some_and(|k| {
            use subtle::ConstantTimeEq;
            expected.as_bytes().ct_eq(k.as_bytes()).into()
        });
        if !ok {
            return Err(ApiError::Unauthorized("无效的访问密钥".into()));
        }
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use axum::{middleware, Router};
    use sea_orm::Database;
    use tower::ServiceExt;

    use super::require_api_key;
    use crate::common::state::AppState;
    use crate::config::Config;

    async fn state_with(key: Option<&str>) -> AppState {
        AppState {
            db: Arc::new(Database::connect("sqlite::memory:").await.unwrap()),
            api_key: key.map(Arc::from),
            wordbooks_cache: Arc::new(parking_lot::Mutex::new(None)),
            shuffle_cache: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            import_cache: Arc::new(parking_lot::Mutex::new(HashMap::new())),
            config: Config::default(),
        }
    }

    fn build(state: AppState) -> Router {
        Router::new()
            .route("/api/wordbooks", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_api_key,
            ))
            .with_state(state)
    }

    fn req() -> Request<Body> {
        Request::builder()
            .uri("/api/wordbooks")
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn passes_through_when_no_key_configured() {
        let app = build(state_with(None).await);
        let resp = app.oneshot(req()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_missing_or_wrong_key() {
        let app = build(state_with(Some("secret123")).await);
        // 无密钥头 → 401
        let resp = app.clone().oneshot(req()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        // 错误的 Bearer → 401
        let r = Request::builder()
            .uri("/api/wordbooks")
            .header(header::AUTHORIZATION, "Bearer wrong")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(r).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn accepts_correct_key() {
        let app = build(state_with(Some("secret123")).await);
        // 正确的 Bearer → 放行
        let r = Request::builder()
            .uri("/api/wordbooks")
            .header(header::AUTHORIZATION, "Bearer secret123")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(r).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // 正确的 X-Api-Key → 放行
        let r = Request::builder()
            .uri("/api/wordbooks")
            .header("x-api-key", "secret123")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(r).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
