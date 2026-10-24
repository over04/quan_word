use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Multipart, Query, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use dict_core::provider_id::ProviderId;

use super::dto::lookup::LookupQuery;
use super::dto::plugin_detail::resp::DictPluginDetailResp;
use super::dto::resp::{DictLookupResp, DictProviderResp};
use super::dto::set_plugin_enabled::SetPluginEnabledReq;
use super::dto::suggest::SuggestQuery;
use super::dto::suggest_resp::DictSuggestResp;
use super::service::DictService;
use super::suggest::SuggestService;
use crate::common::error::ApiError;
use crate::common::http::json::ApiJson;
use crate::common::http::path::ApiPath;
use crate::common::state::AppState;

/// dict 域路由：源列表 + 插件导入/删除/启停 + 聚合抓词（挂在 /api 鉴权层内）。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/dict/providers", get(list_providers))
        .route(
            "/api/dict/plugins",
            post(import_plugin).layer(DefaultBodyLimit::max(300 * 1024)),
        )
        .route(
            "/api/dict/plugins/{id}",
            get(plugin_detail).delete(delete_plugin),
        )
        .route("/api/dict/plugins/{id}/enabled", put(set_plugin_enabled))
        .route("/api/dict/lookup", get(lookup))
        .route("/api/dict/suggest", get(suggest))
}

pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<DictProviderResp>>, ApiError> {
    Ok(Json(DictService::providers(&state.dict)))
}

/// 上传插件包（multipart 字段 file），校验通过后立即装载。
pub async fn import_plugin(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Vec<DictProviderResp>>, ApiError> {
    let mut file: Option<(String, Vec<u8>)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| ApiError::BadRequest(format!("上传失败: {error}")))?
    {
        if field.name() == Some("file") {
            let name = field.file_name().unwrap_or("plugin.json").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|error| ApiError::BadRequest(format!("上传失败: {error}")))?
                .to_vec();
            file = Some((name, bytes));
        }
    }
    let (name, bytes) = file.ok_or_else(|| ApiError::BadRequest("未收到插件文件".into()))?;
    Ok(Json(
        DictService::import_plugin(&state.dict, &name, bytes).await?,
    ))
}

pub async fn plugin_detail(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ProviderId>,
) -> Result<Json<DictPluginDetailResp>, ApiError> {
    Ok(Json(DictService::plugin_detail(&state.dict, &id)?))
}

pub async fn delete_plugin(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ProviderId>,
) -> Result<Json<Vec<DictProviderResp>>, ApiError> {
    Ok(Json(DictService::delete_plugin(&state.dict, &id).await?))
}

pub async fn set_plugin_enabled(
    State(state): State<AppState>,
    ApiPath(id): ApiPath<ProviderId>,
    ApiJson(req): ApiJson<SetPluginEnabledReq>,
) -> Result<Json<DictProviderResp>, ApiError> {
    Ok(Json(DictService::set_enabled(
        &state.dict,
        &id,
        req.enabled,
    )?))
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

/// 建议接口会扇出上游，额外按客户端 IP 限流（60s / 120 次）。
/// 鉴权由全局 `require_api_key` 负责：配置 `server.auth_key` 后同样需要密钥；
/// 未配置密钥时整个 /api 免鉴权，属于未启用鉴权的部署特例。
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
