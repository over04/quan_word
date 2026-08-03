use axum::{extract::State, http::StatusCode, routing::get, Json, Router};

use super::dto::create::CreateWordbookReq;
use super::dto::resp::WordbookResp;
use super::dto::update::UpdateWordbookReq;
use super::service::WordbookService;
use super::tags;
use super::words;
use crate::common::error::ApiError;
use crate::common::http::{json::ApiJson, path::ApiPath};
use crate::common::state::AppState;

/// wordbooks 域路由：本层端点（/api/wordbooks...）+ 聚合子域 words。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/wordbooks", get(list_wordbooks).post(create_wordbook))
        .route(
            "/api/wordbooks/{id}",
            get(get_wordbook)
                .put(update_wordbook)
                .delete(delete_wordbook),
        )
        .merge(words::router::router())
        .merge(tags::router::router())
}

pub async fn list_wordbooks(
    State(state): State<AppState>,
) -> Result<Json<Vec<WordbookResp>>, ApiError> {
    Ok(Json(WordbookService::list(&state).await?))
}

pub async fn get_wordbook(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<i32>,
) -> Result<Json<WordbookResp>, ApiError> {
    Ok(Json(WordbookService::get(&state, id).await?))
}

pub async fn create_wordbook(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateWordbookReq>,
) -> Result<(StatusCode, Json<WordbookResp>), ApiError> {
    let resp = WordbookService::create(&state, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_wordbook(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<i32>,
    ApiJson(req): ApiJson<UpdateWordbookReq>,
) -> Result<Json<WordbookResp>, ApiError> {
    Ok(Json(WordbookService::update(&state, id, req).await?))
}

pub async fn delete_wordbook(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<i32>,
) -> Result<StatusCode, ApiError> {
    WordbookService::delete(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
