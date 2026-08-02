use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use parking_lot::Mutex;
use sea_orm::DatabaseConnection;
use tower_http::compression::CompressionLayer;

use crate::business::wordbooks;
use crate::common::auth;
use crate::common::http::spa::static_handler;
use crate::common::state::AppState;

/// 组装全部路由：聚合各业务域路由（域内递归聚合）+ SPA 静态托管（fallback）。
/// `api_key` 为 config server.auth_key；非 None 时所有 /api 请求需携带密钥。
pub fn build(db: DatabaseConnection, api_key: Option<String>) -> Router {
    let state = AppState {
        db: Arc::new(db),
        api_key: api_key.map(Arc::from),
        wordbooks_cache: Arc::new(Mutex::new(None)),
        shuffle_cache: Arc::new(Mutex::new(HashMap::new())),
    };
    // api 路由挂鉴权层（静态资源不设防，前端 401 时引导输入密钥）
    let api = Router::new().merge(wordbooks::router::router());
    Router::new()
        .merge(api.layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        )))
        .fallback(static_handler)
        // gzip 压缩：客户端 Accept-Encoding: gzip 时对文本/JSON/静态资源生效
        .layer(CompressionLayer::new())
        .with_state(state)
}
