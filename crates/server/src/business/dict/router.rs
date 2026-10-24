use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{ConnectInfo, Query, State};
use axum::routing::get;
use axum::{Json, Router};

use super::dto::lookup::LookupQuery;
use super::dto::resp::{DictLookupResp, DictProviderResp};
use super::dto::suggest::SuggestQuery;
use super::dto::suggest_resp::DictSuggestResp;
use super::service::DictService;
use super::suggest::SuggestService;
use crate::common::error::ApiError;
use crate::common::state::AppState;

/// dict 域路由：源列表 + 聚合抓词（挂在 /api 鉴权层内）。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/dict/providers", get(list_providers))
        .route("/api/dict/lookup", get(lookup))
        .route("/api/dict/suggest", get(suggest))
}

pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<DictProviderResp>>, ApiError> {
    Ok(Json(DictService::providers(&state.dict)))
}

pub async fn lookup(
    State(state): State<AppState>,
    Query(query): Query<LookupQuery>,
) -> Result<Json<DictLookupResp>, ApiError> {
    let refresh = parse_refresh(query.refresh.as_deref());
    let resp = DictService::lookup(
        &state.dict,
        query.word.as_deref(),
        query.providers.as_deref(),
        refresh,
    )
    .await?;
    Ok(Json(resp))
}

/// 建议接口：无鉴权且会扇出上游，按客户端 IP 限流（60s / 40 次）。
/// `ConnectInfo` 依赖 `init.rs` 的 `into_make_service_with_connect_info`。
pub async fn suggest(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(query): Query<SuggestQuery>,
) -> Result<Json<DictSuggestResp>, ApiError> {
    if !state.dict.suggest_limiter.allow(addr.ip(), Instant::now()) {
        return Err(ApiError::TooManyRequests("请求太频繁，请稍后再试".into()));
    }
    let resp = SuggestService::suggest(&state.dict, query.word.as_deref()).await?;
    Ok(Json(resp))
}

/// `refresh` 查询参数：1/true/yes/on 视为强制刷新，其它（含缺省）走缓存。
fn parse_refresh(raw: Option<&str>) -> bool {
    matches!(raw.map(str::trim), Some("1" | "true" | "yes" | "on"))
}
