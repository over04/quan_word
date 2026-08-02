use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::dto::req::create_wordbook_req::CreateWordbookReq;
use crate::dto::req::update_wordbook_req::UpdateWordbookReq;
use crate::dto::resp::wordbook_resp::WordbookResp;
use crate::error::ApiError;
use crate::service::wordbook_service::WordbookService;
use crate::state::AppState;

pub async fn list_wordbooks(
    State(state): State<AppState>,
) -> Result<Json<Vec<WordbookResp>>, ApiError> {
    let resps = WordbookService::list(&state).await?;
    Ok(Json(resps))
}

pub async fn get_wordbook(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<WordbookResp>, ApiError> {
    let resp = WordbookService::get(&state, id).await?;
    Ok(Json(resp))
}

pub async fn create_wordbook(
    State(state): State<AppState>,
    Json(req): Json<CreateWordbookReq>,
) -> Result<(StatusCode, Json<WordbookResp>), ApiError> {
    let resp = WordbookService::create(&state, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_wordbook(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWordbookReq>,
) -> Result<Json<WordbookResp>, ApiError> {
    let resp = WordbookService::update(&state, id, req).await?;
    Ok(Json(resp))
}

pub async fn delete_wordbook(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    WordbookService::delete(&state, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
