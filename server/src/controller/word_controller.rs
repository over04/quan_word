use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::dto::req::create_word_req::CreateWordReq;
use crate::dto::req::update_word_req::UpdateWordReq;
use crate::dto::resp::page_resp::PageResp;
use crate::dto::resp::word_resp::WordResp;
use crate::error::ApiError;
use crate::service::word_service::{WordOrder, WordService};
use crate::state::AppState;

pub async fn list_words(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_page_params(&params)?;
    let order = WordOrder::parse(
        params.get("order").map(String::as_str),
        params.get("seed").map(String::as_str),
    )?;
    let resp = WordService::list(&state.db, book_id, page, page_size, &order).await?;
    Ok(Json(resp))
}

/// 列表模式查询：搜索 + 排序 + 分页。
pub async fn query_words(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_page_params(&params)?;
    let q = params.get("q").cloned();
    let sort = params.get("sort").map(String::as_str).unwrap_or("created_at");
    let order = params.get("order").map(String::as_str).unwrap_or("asc");
    let resp =
        WordService::query(&state.db, book_id, q, sort, order, page, page_size).await?;
    Ok(Json(resp))
}

pub async fn create_word(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Json(req): Json<CreateWordReq>,
) -> Result<(StatusCode, Json<WordResp>), ApiError> {
    let resp = WordService::create(&state.db, book_id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_word(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateWordReq>,
) -> Result<Json<WordResp>, ApiError> {
    let resp = WordService::update(&state.db, id, req).await?;
    Ok(Json(resp))
}

pub async fn delete_word(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, ApiError> {
    WordService::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 解析分页查询参数：page 默认 1；page_size 默认 20，钳制 1..=100。
fn parse_page_params(params: &HashMap<String, String>) -> Result<(u64, u64), ApiError> {
    let page = match params.get("page") {
        None => 1,
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| ApiError::BadRequest("page 必须为正整数".into()))?,
    };
    let page_size = match params.get("page_size") {
        None => 20,
        Some(s) => s
            .parse::<u64>()
            .map_err(|_| ApiError::BadRequest("page_size 必须为正整数".into()))?,
    };
    if page == 0 {
        return Err(ApiError::BadRequest("page 必须为正整数".into()));
    }
    Ok((page, page_size.clamp(1, 100)))
}
