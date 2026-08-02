use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};

use super::dto::create::CreateWordReq;
use super::dto::list::ListWordsQuery;
use super::dto::resp::WordResp;
use super::dto::search::SearchWordsQuery;
use super::dto::update::UpdateWordReq;
use super::order::WordOrder;
use super::service::WordService;
use super::sort::SortField;
use super::sort_dir::SortDir;
use crate::common::error::ApiError;
use crate::common::model::page::PageResp;
use crate::common::model::paging::parse_paging;
use crate::common::state::AppState;

/// words 子域路由：全部挂在 /api/wordbooks/{book_id}/words 之下。
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/wordbooks/{book_id}/words",
            get(list_words).post(create_word),
        )
        .route("/api/wordbooks/{book_id}/words/query", get(query_words))
        .route(
            "/api/wordbooks/{book_id}/words/{id}",
            put(update_word).delete(delete_word),
        )
}

pub async fn list_words(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Query(query): Query<ListWordsQuery>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_paging(query.page.as_deref(), query.page_size.as_deref())?;
    let order = WordOrder::parse(query.order.as_deref(), query.seed.as_deref())?;
    Ok(Json(
        WordService::list(&state, book_id, page, page_size, &order).await?,
    ))
}

/// 列表模式查询：搜索 + 排序 + 分页。
pub async fn query_words(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Query(query): Query<SearchWordsQuery>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_paging(query.page.as_deref(), query.page_size.as_deref())?;
    let field = SortField::parse(query.sort.as_deref().unwrap_or("created_at"))?;
    let dir = SortDir::parse(query.order.as_deref().unwrap_or("asc"))?;
    Ok(Json(
        WordService::query(&state, book_id, query.q, field, dir, page, page_size).await?,
    ))
}

pub async fn create_word(
    State(state): State<AppState>,
    Path(book_id): Path<i32>,
    Json(req): Json<CreateWordReq>,
) -> Result<(StatusCode, Json<WordResp>), ApiError> {
    let resp = WordService::create(&state, book_id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_word(
    State(state): State<AppState>,
    Path((book_id, id)): Path<(i32, i32)>,
    Json(req): Json<UpdateWordReq>,
) -> Result<Json<WordResp>, ApiError> {
    Ok(Json(WordService::update(&state, book_id, id, req).await?))
}

pub async fn delete_word(
    State(state): State<AppState>,
    Path((book_id, id)): Path<(i32, i32)>,
) -> Result<StatusCode, ApiError> {
    WordService::delete(&state, book_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
