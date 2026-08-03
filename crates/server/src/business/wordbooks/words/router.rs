use axum::{
    extract::{DefaultBodyLimit, Multipart, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post, put},
    Json, Router,
};

use super::dto::batch::BatchDeleteWordsReq;
use super::dto::batch::BatchDeleteWordsResp;
use super::dto::batch_tag::BatchTagWordsReq;
use super::dto::batch_tag::BatchTagWordsResp;
use super::dto::create::CreateWordReq;
use super::dto::import::{
    ImportExecReq, ImportPreviewResp, ImportResp, ImportRowsReq, ImportRowsResp, PreviewPageQuery,
};
use super::dto::list::ListWordsQuery;
use super::dto::resp::WordResp;
use super::dto::search::SearchWordsQuery;
use super::dto::template::TemplateQuery;
use super::dto::update::UpdateWordReq;
use super::dto::update_tags::UpdateWordTagsReq;
use super::error::WordError;
use super::order::WordOrder;
use super::service::WordService;
use super::sort::SortField;
use super::sort_dir::SortDir;
use crate::common::error::ApiError;
use crate::common::http::{json::ApiJson, path::ApiPath};
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
            "/api/wordbooks/{book_id}/words/template",
            get(download_template),
        )
        .route(
            "/api/wordbooks/{book_id}/words/import/preview",
            post(import_preview).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route(
            "/api/wordbooks/{book_id}/words/import/rows",
            post(page_rows),
        )
        .route(
            "/api/wordbooks/{book_id}/words/import",
            post(import_words).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
        )
        .route(
            "/api/wordbooks/{book_id}/words/batch-delete",
            post(batch_delete_words),
        )
        .route(
            "/api/wordbooks/{book_id}/words/batch-tag",
            post(batch_tag_words),
        )
        .route(
            "/api/wordbooks/{book_id}/words/{id}",
            put(update_word).delete(delete_word),
        )
        .route(
            "/api/wordbooks/{book_id}/words/{id}/tags",
            put(update_word_tags),
        )
}

pub async fn list_words(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    Query(query): Query<ListWordsQuery>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_paging(query.page.as_deref(), query.page_size.as_deref())?;
    let order = WordOrder::parse(query.order.as_deref(), query.seed.as_deref())?;
    let tag_ids = WordService::parse_tag_ids(query.tag.as_deref())?;
    Ok(Json(
        WordService::list(&state, book_id, page, page_size, &order, &tag_ids).await?,
    ))
}

/// 列表模式查询：搜索 + 排序 + 分页。
pub async fn query_words(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    Query(query): Query<SearchWordsQuery>,
) -> Result<Json<PageResp<WordResp>>, ApiError> {
    let (page, page_size) = parse_paging(query.page.as_deref(), query.page_size.as_deref())?;
    let field = SortField::parse(query.sort.as_deref().unwrap_or("created_at"))?;
    let dir = SortDir::parse(query.order.as_deref().unwrap_or("asc"))?;
    let tag_ids = WordService::parse_tag_ids(query.tag.as_deref())?;
    Ok(Json(
        WordService::query(
            &state, book_id, query.q, field, dir, page, page_size, &tag_ids,
        )
        .await?,
    ))
}

pub async fn create_word(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    ApiJson(req): ApiJson<CreateWordReq>,
) -> Result<(StatusCode, Json<WordResp>), ApiError> {
    let resp = WordService::create(&state, book_id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

pub async fn update_word(
    State(state): State<AppState>,
    ApiPath((book_id, id)): ApiPath<(i32, i32)>,
    ApiJson(req): ApiJson<UpdateWordReq>,
) -> Result<Json<WordResp>, ApiError> {
    Ok(Json(WordService::update(&state, book_id, id, req).await?))
}

pub async fn delete_word(
    State(state): State<AppState>,
    ApiPath((book_id, id)): ApiPath<(i32, i32)>,
) -> Result<StatusCode, ApiError> {
    WordService::delete(&state, book_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 替换单词标签集（全量）。
pub async fn update_word_tags(
    State(state): State<AppState>,
    ApiPath((book_id, id)): ApiPath<(i32, i32)>,
    ApiJson(req): ApiJson<UpdateWordTagsReq>,
) -> Result<Json<WordResp>, ApiError> {
    Ok(Json(
        WordService::update_tags(&state, book_id, id, req).await?,
    ))
}

/// 下载导入模板：format=csv|xlsx，缺省 csv。
pub async fn download_template(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    Query(query): Query<TemplateQuery>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    // 校验单词书存在（404 提示更友好）
    WordService::book_exists(&state, book_id).await?;
    let format = query.format.as_deref().unwrap_or("csv");
    match format {
        "csv" => {
            let body = WordService::template_csv();
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/csv; charset=utf-8"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"words_template.csv\""),
            );
            Ok((headers, body))
        }
        "xlsx" => {
            let body = WordService::template_xlsx()?;
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                ),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_static("attachment; filename=\"words_template.xlsx\""),
            );
            Ok((headers, body))
        }
        other => Err(ApiError::from(WordError::UnsupportedFormat {
            ext: other.to_string(),
        })),
    }
}

/// 上传文件解析预览：multipart 字段 file，接受 csv / xlsx / xls / ods。不落库。
pub async fn import_preview(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    Query(query): Query<PreviewPageQuery>,
    mut multipart: Multipart,
) -> Result<Json<ImportPreviewResp>, ApiError> {
    let (name, bytes) = extract_file(&mut multipart).await?;
    Ok(Json(
        WordService::import_preview(
            &state,
            book_id,
            &name,
            bytes,
            query.page.unwrap_or(1),
            query.page_size.unwrap_or(25),
        )
        .await?,
    ))
}

/// 行分页/编辑/筛选：会话内应用修正 → 重新校验 → 返回当前页。
pub async fn page_rows(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    ApiJson(req): ApiJson<ImportRowsReq>,
) -> Result<Json<ImportRowsResp>, ApiError> {
    Ok(Json(WordService::page_rows(&state, book_id, req).await?))
}

/// 导入执行：JSON body 为 token 会话 + 重复行策略（update_rows）。
pub async fn import_words(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    ApiJson(req): ApiJson<ImportExecReq>,
) -> Result<Json<ImportResp>, ApiError> {
    Ok(Json(
        WordService::import_words(&state, book_id, req).await?,
    ))
}

/// 从 multipart 提取 file 字段（两导入接口共用）。
async fn extract_file(multipart: &mut Multipart) -> Result<(String, Vec<u8>), ApiError> {
    let mut file: Option<(String, Vec<u8>)> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("上传失败: {e}")))?
    {
        if field.name() == Some("file") {
            let name = field.file_name().unwrap_or("").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("上传失败: {e}")))?
                .to_vec();
            file = Some((name, bytes));
        }
    }
    file.ok_or_else(|| ApiError::BadRequest("未收到文件".into()))
}

/// 批量删除单词（限定归属该书）。
pub async fn batch_delete_words(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    ApiJson(req): ApiJson<BatchDeleteWordsReq>,
) -> Result<Json<BatchDeleteWordsResp>, ApiError> {
    Ok(Json(
        WordService::batch_delete(&state, book_id, req.ids).await?,
    ))
}

/// 批量给单词打标签（限定归属该书，只添加）。
pub async fn batch_tag_words(
    State(state): State<AppState>,
    ApiPath(book_id): ApiPath<i32>,
    ApiJson(req): ApiJson<BatchTagWordsReq>,
) -> Result<Json<BatchTagWordsResp>, ApiError> {
    Ok(Json(WordService::batch_tag(&state, book_id, req).await?))
}
