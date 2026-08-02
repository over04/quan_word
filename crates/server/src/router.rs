use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use parking_lot::Mutex;
use sea_orm::DatabaseConnection;
use tower_http::compression::CompressionLayer;

use crate::business::wordbooks;
use crate::common::http::spa::static_handler;
use crate::common::state::AppState;

/// 组装全部路由：聚合各业务域路由（域内递归聚合）+ SPA 静态托管（fallback）。
pub fn build(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        wordbooks_cache: Arc::new(Mutex::new(None)),
        shuffle_cache: Arc::new(Mutex::new(HashMap::new())),
    };
    Router::new()
        .merge(wordbooks::router::router())
        .fallback(static_handler)
        // gzip 压缩：客户端 Accept-Encoding: gzip 时对文本/JSON/静态资源生效
        .layer(CompressionLayer::new())
        .with_state(state)
}
