use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};

use super::dto::create::CreateTagReq;
use super::dto::resp::TagResp;
use super::dto::update::UpdateTagReq;
use super::service::TagService;
use crate::common::error::ApiError;
use crate::common::state::AppState;

/// tags 子域路由：全部挂在 /api/wordbooks/{book_id}/tags 之下。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/wordbooks/{book_id}/tags",
            get(list_tags).post(create_tag),
        )
        .route(
            "/api/wordbooks/{book_id}/tags/{id}",
            put(update_tag).delete(delete_tag),
        )
}

pub async fn list_tags(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
) -> Result<Json<Vec<TagResp>>, ApiError> {
    Ok(Json(TagService::list(&state, book_id).await?))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Json(req): Json<CreateTagReq>,
) -> Result<(StatusCode, Json<TagResp>), ApiError> {
    let resp = TagService::create(&state, book_id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path((book_id, id)): Path<(i32, i32)>,
    Json(req): Json<UpdateTagReq>,
) -> Result<Json<TagResp>, ApiError> {
    Ok(Json(TagService::update(&state, book_id, id, req).await?))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path((book_id, id)): Path<(i32, i32)>,
) -> Result<StatusCode, ApiError> {
    TagService::delete(&state, book_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
