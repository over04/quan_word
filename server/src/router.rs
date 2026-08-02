use std::sync::Arc;

use axum::{
    Router,
    routing::{get, put},
};
use sea_orm::DatabaseConnection;

use crate::controller::static_controller::handler::static_handler;
use crate::controller::word_controller;
use crate::controller::wordbook_controller;
use crate::state::AppState;

/// 组装全部路由：API 路由 + SPA 静态托管（fallback）。
pub fn build(db: DatabaseConnection) -> Router {
    let state = AppState { db: Arc::new(db) };
    Router::new()
        .route(
            "/api/wordbooks",
            get(wordbook_controller::list_wordbooks).post(wordbook_controller::create_wordbook),
        )
        .route(
            "/api/wordbooks/{id}",
            put(wordbook_controller::update_wordbook).delete(wordbook_controller::delete_wordbook),
        )
        .route(
            "/api/wordbooks/{id}/words",
            get(word_controller::list_words).post(word_controller::create_word),
        )
        .route(
            "/api/wordbooks/{id}/words/query",
            get(word_controller::query_words),
        )
        .route(
            "/api/words/{id}",
            put(word_controller::update_word).delete(word_controller::delete_word),
        )
        .fallback(static_handler)
        .with_state(state)
}
