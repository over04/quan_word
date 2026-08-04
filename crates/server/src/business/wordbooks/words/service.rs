use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use chrono::Utc;
use entity::definition::Definition;
use entity::word;
use sea_orm::{DatabaseConnection, Set};

use super::dto::batch::BatchDeleteWordsResp;
use super::dto::batch_tag::BatchTagWordsReq;
use super::dto::batch_tag::BatchTagWordsResp;
use super::dto::create::CreateWordReq;
use super::dto::import::{
    ImportExecReq, ImportPreviewResp, ImportResp, ImportRowData, ImportRowView, ImportRowsReq,
    ImportRowsResp,
};
use super::dto::resp::WordResp;
use super::dto::update::UpdateWordReq;
use super::dto::update_tags::UpdateWordTagsReq;
use super::error::WordError;
use super::import;
use super::order::WordOrder;
use super::repo::WordRepo;
use super::sort::SortField;
use super::sort_dir::SortDir;
use super::tag_match::TagMatch;
use crate::common::model::page::PageResp;
use crate::common::state::{AppState, ImportCacheEntry, SHUFFLE_CACHE_CAP};

/// 导入会话 token 序号（与纳秒时间戳拼合防碰撞；token 非安全边界，仅会话凭据）。
static IMPORT_TOKEN_SEQ: AtomicU64 = AtomicU64::new(0);

/// 生成导入预览会话 token。
fn new_import_token() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("系统时间应晚于 1970")
        .as_nanos();
    let seq = IMPORT_TOKEN_SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}-{seq:x}")
}

/// 导入解析中间结果（预览与执行共用）。
struct ImportPlan {
    groups: Vec<import::WordGroup>,     // 按拼写合并后的单词
    errors: Vec<(usize, String)>,       // 行错误（行号升序）
    new_tags: Vec<String>,              // 缺失标签名（跨组去重，保持出现顺序）
    existing_tags: u64,
    duplicates: Vec<(usize, String)>,   // 重复组（组首行行号, 拼写）
}

/// 单词业务逻辑：分页查询（含排序/打乱）/ 搜索查询 / 创建 / 更新 / 删除。
pub struct WordService;

impl WordService {
    pub async fn list(
        state: &AppState,
        book_id: i32,
        page: u64,
        page_size: u64,
        order: &WordOrder,
        tag_match: TagMatch,
        tag_ids: &[i32],
    ) -> Result<PageResp<WordResp>, WordError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        match order {
            // SQL 层排序分页：id / 字母序
            WordOrder::IdAsc => {
                let (models, total) = WordRepo::browse_page(
                    db,
                    book_id,
                    word::Column::Id,
                    SortDir::Asc,
                    page,
                    page_size,
                    tag_ids,
                    tag_match,
                )
                .await?;
                Self::to_page_with_tags(db, models, total, page, page_size).await
            }
            WordOrder::IdDesc => {
                let (models, total) = WordRepo::browse_page(
                    db,
                    book_id,
                    word::Column::Id,
                    SortDir::Desc,
                    page,
                    page_size,
                    tag_ids,
                    tag_match,
                )
                .await?;
                Self::to_page_with_tags(db, models, total, page, page_size).await
            }
            WordOrder::Spelling => {
                let (models, total) = WordRepo::browse_page(
                    db,
                    book_id,
                    word::Column::Spelling,
                    SortDir::Asc,
                    page,
                    page_size,
                    tag_ids,
                    tag_match,
                )
                .await?;
                Self::to_page_with_tags(db, models, total, page, page_size).await
            }
            // 打乱：全量 id 按 seed 确定性洗牌（跨库一致），按页取 id 切片后查单词
            WordOrder::Random(seed) => {
                // 洗牌序列缓存：(book_id, 筛选标签 ids, 匹配模式, seed) → 完整 id 序列，避免每页请求重复全量洗牌
                // 注意：锁 guard 立即 drop，禁止跨 await 持有（parking_lot guard 非 Send）
                let key = (book_id, tag_ids.to_vec(), tag_match.cache_code().to_owned(), seed.clone());
                let cached = state.shuffle_cache.lock().get(&key).cloned();
                let ordered = match cached {
                    Some(v) => v,
                    None => {
                        let mut ordered = WordRepo::find_all_ids(db, book_id, tag_ids, tag_match).await?;
                        Self::seeded_shuffle(&mut ordered, seed);
                        let mut m = state.shuffle_cache.lock();
                        if m.len() >= SHUFFLE_CACHE_CAP {
                            m.clear();
                        }
                        m.insert(key, ordered.clone());
                        ordered
                    }
                };
                let total = ordered.len() as u64;
                let slice: Vec<i32> = ordered
                    .iter()
                    .skip(((page - 1) * page_size) as usize)
                    .take(page_size as usize)
                    .copied()
                    .collect();
                let models = if slice.is_empty() {
                    Vec::new()
                } else {
                    WordRepo::find_by_ids(db, book_id, &slice).await?
                };
                // `IN` 查询结果无序：按切片顺序重排
                let by_id: HashMap<i32, word::Model> =
                    models.into_iter().map(|m| (m.id, m)).collect();
                let ordered_models: Vec<word::Model> = slice
                    .iter()
                    .filter_map(|id| by_id.get(id).cloned())
                    .collect();
                Self::to_page_with_tags(db, ordered_models, total, page, page_size).await
            }
        }
    }

    /// 列表模式查询：书内搜索（拼写/释义模糊匹配）+ 白名单排序 + 标签筛选（交集/并集）+ 分页。
    #[allow(clippy::too_many_arguments)]
    pub async fn query(
        state: &AppState,
        book_id: i32,
        q: Option<String>,
        field: SortField,
        dir: SortDir,
        page: u64,
        page_size: u64,
        tag_match: TagMatch,
        tag_ids: &[i32],
    ) -> Result<PageResp<WordResp>, WordError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let q = q.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let (models, total) =
            WordRepo::search_page(db, book_id, q, field, dir, page, page_size, tag_ids, tag_match)
                .await?;
        Self::to_page_with_tags(db, models, total, page, page_size).await
    }

    pub async fn create(
        state: &AppState,
        book_id: i32,
        req: CreateWordReq,
    ) -> Result<WordResp, WordError> {
        Self::validate(&req.spelling, &req.definitions)?;
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let tag_ids = Self::validated_tag_ids(db, book_id, &req.tags).await?;
        let now = Utc::now();
        let (model, tag_ids) = WordRepo::insert_with_tags(
            db,
            word::ActiveModel {
                wordbook_id: Set(book_id),
                spelling: Set(req.spelling),
                phonetic: Set(req.phonetic),
                definitions: Set(serde_json::to_value(&req.definitions)?),
                example: Set(req.example),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            },
            &tag_ids,
        )
        .await?;
        // 新词影响列表页 word_count 与洗牌 id 集合：两个缓存都失效
        state.invalidate_wordbooks();
        state.shuffle_cache.lock().clear();
        Self::to_resp(&model, &tag_ids)
    }

    pub async fn update(
        state: &AppState,
        book_id: i32,
        id: i32,
        req: UpdateWordReq,
    ) -> Result<WordResp, WordError> {
        Self::validate(&req.spelling, &req.definitions)?;
        let db = state.db.as_ref();
        let w = WordRepo::find_by_id(db, id)
            .await?
            .ok_or(WordError::WordNotFound { word_id: id })?;
        if w.wordbook_id != book_id {
            return Err(WordError::WordNotInWordbook {
                word_id: id,
                wordbook_id: book_id,
            });
        }
        let tag_ids = Self::validated_tag_ids(db, book_id, &req.tags).await?;
        let mut model: word::ActiveModel = w.into();
        model.spelling = Set(req.spelling);
        model.phonetic = Set(req.phonetic);
        model.definitions = Set(serde_json::to_value(&req.definitions)?);
        model.example = Set(req.example);
        model.updated_at = Set(Utc::now());
        let (saved, tag_ids) = WordRepo::update_with_tags(db, model, &tag_ids).await?;
        // 标签变化影响筛选与打乱结果集合：洗牌缓存失效
        state.shuffle_cache.lock().clear();
        Self::to_resp(&saved, &tag_ids)
    }

    pub async fn delete(state: &AppState, book_id: i32, id: i32) -> Result<(), WordError> {
        let w = WordRepo::find_by_id(state.db.as_ref(), id)
            .await?
            .ok_or(WordError::WordNotFound { word_id: id })?;
        if w.wordbook_id != book_id {
            return Err(WordError::WordNotInWordbook {
                word_id: id,
                wordbook_id: book_id,
            });
        }
        let rows = WordRepo::delete_by_id(state.db.as_ref(), id).await?;
        if rows == 0 {
            return Err(WordError::WordNotFound { word_id: id });
        }
        // 删除影响列表页 word_count 与洗牌 id 集合：两个缓存都失效
        state.invalidate_wordbooks();
        state.shuffle_cache.lock().clear();
        Ok(())
    }

    /// 解析行数据并构建导入计划：行级校验分类 → 按拼写分组 → 标签差异、重复判定（预览与执行共用）。
    async fn build_import_plan(
        state: &AppState,
        book_id: i32,
        rows: &[import::RowData],
    ) -> Result<ImportPlan, WordError> {
        let db = state.db.as_ref();
        let (prepared, errors) = import::prepare_rows(rows);
        let groups = import::group_rows(&prepared);
        let tag_map = WordRepo::find_tag_map(db, book_id).await?;
        // 文件标签名全集（跨组去重，保持出现顺序）
        let mut all: Vec<String> = Vec::new();
        for g in &groups {
            for name in &g.tag_names {
                if !all.contains(name) {
                    all.push(name.clone());
                }
            }
        }
        let mut new_tags = Vec::new();
        let mut existing_tags = 0u64;
        for name in all {
            if tag_map.contains_key(&name) {
                existing_tags += 1;
            } else {
                new_tags.push(name);
            }
        }
        let spellings = WordRepo::find_spellings(db, book_id).await?;
        let mut duplicates = Vec::new();
        for g in &groups {
            if spellings.contains_key(&g.spelling.to_lowercase()) {
                duplicates.push((g.row_nos[0], g.spelling.clone()));
            }
        }
        Ok(ImportPlan {
            groups,
            errors,
            new_tags,
            existing_tags,
            duplicates,
        })
    }

    /// 批量导入预览：解析文件 → 缓存会话 → 返回统计 + 第一页行视图（不落库）。
    pub async fn import_preview(
        state: &AppState,
        book_id: i32,
        file_name: &str,
        bytes: Vec<u8>,
        page: u64,
        page_size: u64,
    ) -> Result<ImportPreviewResp, WordError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_string();
        let rows = import::parse_file(&bytes, &ext, state.config.import.max_rows)?;
        let (views, total, invalid, dup_total, dup_groups, new_tags, existing_tags) =
            Self::build_views(state, book_id, &rows).await?;
        let (page_rows, total_pages) = Self::page_groups(views, page, page_size);
        let token = new_import_token();
        // 缓存全部解析行（含全空行，行号 = 索引 + 2；updates 映射依赖此不变量）
        let dto_rows: Vec<ImportRowData> = rows.iter().map(|r| r.to_dto()).collect();
        let mut m = state.import_cache.lock();
        m.retain(|_, e| {
            e.created_at
                .elapsed()
                < Duration::from_secs(state.config.import.cache_ttl_secs)
        });
        if m.len() >= state.config.import.cache_cap {
            m.clear();
        }
        m.insert(
            token.clone(),
            ImportCacheEntry {
                book_id,
                rows: serde_json::to_vec(&dto_rows)?.into(),
                created_at: Instant::now(),
            },
        );
        drop(m);
        Ok(ImportPreviewResp {
            token,
            total_rows: total,
            valid_rows: total - invalid,
            invalid_rows: invalid,
            duplicate_total: dup_total,
            duplicate_groups: dup_groups,
            rows: page_rows,
            new_tags,
            existing_tags,
            page,
            page_size,
            total_pages,
        })
    }

    /// 构建全量行视图与统计（预览与重新校验共用；不落库、不建会话）。
    #[allow(clippy::type_complexity)]
    async fn build_views(
        state: &AppState,
        book_id: i32,
        rows: &[import::RowData],
    ) -> Result<
        (
            Vec<ImportRowView>,
            u64,
            u64,
            u64,
            Vec<u64>,
            Vec<String>,
            u64,
        ),
        WordError,
    > {
        let plan = Self::build_import_plan(state, book_id, rows).await?;
        let err_map: HashMap<usize, String> = plan.errors.into_iter().collect();
        let dup_first: HashSet<usize> = plan.duplicates.iter().map(|(r, _)| *r).collect();
        let dup_groups: Vec<u64> = plan
            .duplicates
            .iter()
            .map(|(r, _)| *r as u64)
            .collect();
        let mut group_map: HashMap<usize, usize> = HashMap::new(); // 行号 → 组首行号
        for g in &plan.groups {
            let first = g.row_nos[0];
            for &rn in &g.row_nos {
                group_map.insert(rn, first);
            }
        }
        let mut views = Vec::new();
        for raw in rows {
            if raw.is_blank() {
                continue;
            }
            let row_no = raw.row_no;
            let group_first = group_map.get(&row_no).copied().unwrap_or(row_no);
            views.push(ImportRowView {
                row: row_no as u64,
                spelling: raw.spelling.clone(),
                phonetic: raw.phonetic.clone(),
                pos: raw.pos.clone(),
                meaning: raw.meaning.clone(),
                example: raw.example.clone(),
                tags: raw.tags.clone(),
                error: err_map.get(&row_no).cloned(),
                is_duplicate: dup_first.contains(&group_first),
                group: group_first as u64,
            });
        }
        let total = views.len() as u64;
        let invalid = err_map.len() as u64;
        Ok((
            views,
            total,
            invalid,
            dup_first.len() as u64,
            dup_groups,
            plan.new_tags,
            plan.existing_tags,
        ))
    }

    /// 按组切片（组不跨页，页序 = 组序）：返回（当前页展开行, 总页数）。
    fn page_groups(views: Vec<ImportRowView>, page: u64, page_size: u64) -> (Vec<ImportRowView>, u64) {
        let mut group_idx: HashMap<u64, usize> = HashMap::new();
        let mut groups: Vec<Vec<ImportRowView>> = Vec::new();
        for v in views {
            if let Some(&i) = group_idx.get(&v.group) {
                groups[i].push(v);
            } else {
                group_idx.insert(v.group, groups.len());
                groups.push(vec![v]);
            }
        }
        let total_pages = (groups.len() as u64).div_ceil(page_size);
        let start = ((page - 1) * page_size) as usize;
        let page_rows: Vec<ImportRowView> = groups
            .iter()
            .skip(start)
            .take(page_size as usize)
            .flatten()
            .cloned()
            .collect();
        (page_rows, total_pages)
    }

    /// 行分页/编辑/筛选：会话内应用行级修正 → 重新校验 → 按筛选分页返回（不消费会话）。
    pub async fn page_rows(
        state: &AppState,
        book_id: i32,
        req: ImportRowsReq,
    ) -> Result<ImportRowsResp, WordError> {
        Self::ensure_book_exists(state.db.as_ref(), book_id).await?;
        let page = req.page.max(1);
        let page_size = req.page_size.clamp(1, 100);
        let ttl = Duration::from_secs(state.config.import.cache_ttl_secs);
        // 会话内更新行数据（不消费会话；编辑/翻页/筛选复用同一 token）。
        // 锁与借用限定在块内，保证不跨 await（parking_lot guard 非 Send）。
        let dto_rows: Vec<ImportRowData> = {
            let mut m = state.import_cache.lock();
            let entry = match m.get_mut(&req.token) {
                Some(e) if e.book_id == book_id && e.created_at.elapsed() < ttl => e,
                _ => return Err(WordError::ImportSessionInvalid),
            };
            let mut dto_rows: Vec<ImportRowData> = serde_json::from_slice(&entry.rows)?;
            for fix in &req.updates {
                // 行号 < 2 不可能指向数据行，显式忽略（防 usize 减法下溢 panic）
                if fix.row < 2 {
                    continue;
                }
                let idx = fix.row as usize - 2;
                if let Some(r) = dto_rows.get_mut(idx) {
                    *r = fix.clone();
                }
            }
            entry.rows = serde_json::to_vec(&dto_rows)?.into();
            dto_rows
        };
        let rows: Vec<import::RowData> = dto_rows.iter().map(import::RowData::from_dto).collect();
        let (views, total, invalid, dup_total, dup_groups, new_tags, existing_tags) =
            Self::build_views(state, book_id, &rows).await?;
        let filtered: Vec<ImportRowView> = match req.filter.as_str() {
            "all" => views,
            "error" => views.into_iter().filter(|r| r.error.is_some()).collect(),
            "duplicate" => views.into_iter().filter(|r| r.is_duplicate).collect(),
            other => {
                return Err(WordError::InvalidImportFilter {
                    filter: other.into(),
                });
            }
        };
        // 按组切片：组不跨页（前端按组卡片展示），页序 = 组序
        let (page_rows, total_pages) = Self::page_groups(filtered, page, page_size);
        Ok(ImportRowsResp {
            total_rows: total,
            valid_rows: total - invalid,
            invalid_rows: invalid,
            duplicate_total: dup_total,
            duplicate_groups: dup_groups,
            rows: page_rows,
            new_tags,
            existing_tags,
            page,
            page_size,
            total_pages,
        })
    }

    /// 批量导入执行：取预览会话 → 应用修正 → 重新校验 → 事务落库（容错：错误行跳过）。
    pub async fn import_words(
        state: &AppState,
        book_id: i32,
        req: ImportExecReq,
    ) -> Result<ImportResp, WordError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let ttl = Duration::from_secs(state.config.import.cache_ttl_secs);
        // 一次性消费会话（不存在/过期/跨书统一报错，不泄露差异）
        let entry = {
            let mut m = state.import_cache.lock();
            m.remove(&req.token)
        };
        let entry = match entry {
            Some(e) if e.book_id == book_id && e.created_at.elapsed() < ttl => e,
            _ => return Err(WordError::ImportSessionInvalid),
        };
        let dto_rows: Vec<ImportRowData> = serde_json::from_slice(&entry.rows)?;
        let rows: Vec<import::RowData> = dto_rows
            .iter()
            .map(import::RowData::from_dto)
            .collect();
        let plan = Self::build_import_plan(state, book_id, &rows).await?;
        if plan.groups.is_empty() {
            return Ok(ImportResp {
                imported: 0,
                updated: 0,
                skipped_errors: plan.errors.len() as u64,
                skipped_duplicates: 0,
                created_tags: 0,
            });
        }
        // 标签：缺失名批量创建（已存在的复用；过滤预览后并发创建的，避免撞 UNIQUE 约束）
        let mut tag_map = WordRepo::find_tag_map(db, book_id).await?;
        let to_create: Vec<String> = plan
            .new_tags
            .iter()
            .filter(|n| !tag_map.contains_key(*n))
            .cloned()
            .collect();
        tag_map.extend(WordRepo::insert_tags(db, book_id, &to_create).await?);
        // 重复判定以执行时最新数据为准
        let spellings = WordRepo::find_spellings(db, book_id).await?;
        let now = Utc::now();
        let update_set: HashSet<usize> = req.update_rows.iter().map(|r| *r as usize).collect();
        let mut inserts: Vec<(word::ActiveModel, Vec<i32>)> = Vec::new();
        let mut update_targets: Vec<(usize, i32, Vec<i32>)> = Vec::new(); // (组首行号, existing_id, file_tag_ids)
        let mut dup_total = 0u64; // 执行时的重复组总数（更新 + 跳过）
        for group in &plan.groups {
            let tag_ids: Vec<i32> = group
                .tag_names
                .iter()
                .map(|n| *tag_map.get(n).expect("标签映射应完整"))
                .collect();
            if let Some(existing_id) = spellings.get(&group.spelling.to_lowercase()) {
                dup_total += 1;
                // 组内任一行被勾选「更新」→ 整组更新
                if group.row_nos.iter().any(|rn| update_set.contains(rn)) {
                    update_targets.push((group.row_nos[0], *existing_id, tag_ids));
                }
            } else {
                inserts.push((
                    word::ActiveModel {
                        wordbook_id: Set(book_id),
                        spelling: Set(group.spelling.clone()),
                        phonetic: Set(group.phonetic.clone()),
                        definitions: Set(serde_json::to_value(&group.definitions)?),
                        example: Set(group.example.clone()),
                        created_at: Set(now),
                        updated_at: Set(now),
                        ..Default::default()
                    },
                    tag_ids,
                ));
            }
        }
        // 现有标签（合并用）：一次查询全部更新目标，避免 N+1
        let dup_word_ids: Vec<i32> = update_targets.iter().map(|(_, id, _)| *id).collect();
        let existing_links = WordRepo::find_tag_ids_by_word_ids(db, &dup_word_ids).await?;
        let mut updates: Vec<(word::ActiveModel, Vec<i32>)> =
            Vec::with_capacity(update_targets.len());
        for (first_row, word_id, file_tag_ids) in update_targets {
            let group = plan
                .groups
                .iter()
                .find(|g| g.row_nos[0] == first_row)
                .expect("组首行号来自 plan.groups");
            // 标签合并：现有 ∪ 文件，去重升序
            let mut merged = existing_links.get(&word_id).cloned().unwrap_or_default();
            merged.extend(file_tag_ids);
            merged.sort_unstable();
            merged.dedup();
            let existing = WordRepo::find_by_id(db, word_id)
                .await?
                .ok_or(WordError::WordNotFound { word_id })?;
            let mut model: word::ActiveModel = existing.into();
            model.spelling = Set(group.spelling.clone());
            model.phonetic = Set(group.phonetic.clone());
            model.definitions = Set(serde_json::to_value(&group.definitions)?);
            model.example = Set(group.example.clone());
            model.updated_at = Set(now);
            updates.push((model, merged));
        }
        WordRepo::import_inserts(db, &inserts, &updates).await?;
        // 新词影响列表页 word_count 与洗牌 id 集合：两个缓存都失效
        state.invalidate_wordbooks();
        state.shuffle_cache.lock().clear();
        Ok(ImportResp {
            imported: inserts.len() as u64,
            updated: updates.len() as u64,
            skipped_errors: plan.errors.len() as u64,
            skipped_duplicates: dup_total - updates.len() as u64,
            created_tags: plan.new_tags.len() as u64,
        })
    }

    /// 批量删除（校验归属该书）；返回实际删除数。
    pub async fn batch_delete(
        state: &AppState,
        book_id: i32,
        ids: Vec<i32>,
    ) -> Result<BatchDeleteWordsResp, WordError> {
        if ids.is_empty() {
            return Err(WordError::EmptySelection);
        }
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let deleted = WordRepo::batch_delete(db, book_id, &ids).await?;
        // 删除影响列表页 word_count 与洗牌 id 集合：两个缓存都失效
        state.invalidate_wordbooks();
        state.shuffle_cache.lock().clear();
        Ok(BatchDeleteWordsResp { deleted })
    }

    /// 替换单词标签集（全量）：校验归属后重建关联；返回更新后的单词。
    pub async fn update_tags(
        state: &AppState,
        book_id: i32,
        id: i32,
        req: UpdateWordTagsReq,
    ) -> Result<WordResp, WordError> {
        let db = state.db.as_ref();
        let w = WordRepo::find_by_id(db, id)
            .await?
            .ok_or(WordError::WordNotFound { word_id: id })?;
        if w.wordbook_id != book_id {
            return Err(WordError::WordNotInWordbook {
                word_id: id,
                wordbook_id: book_id,
            });
        }
        let tag_ids = Self::validated_tag_ids(db, book_id, &req.tags).await?;
        let mut model: word::ActiveModel = w.into();
        model.updated_at = Set(Utc::now());
        let (saved, tag_ids) = WordRepo::update_with_tags(db, model, &tag_ids).await?;
        // 标签关系变化影响筛选后的打乱结果集合：洗牌缓存失效（词数不变，无需刷新 wordbooks 缓存）
        state.shuffle_cache.lock().clear();
        Self::to_resp(&saved, &tag_ids)
    }

    /// 批量给单词打标签（只添加，不清除已有标签）；返回实际新增关联数。
    pub async fn batch_tag(
        state: &AppState,
        book_id: i32,
        req: BatchTagWordsReq,
    ) -> Result<BatchTagWordsResp, WordError> {
        if req.word_ids.is_empty() {
            return Err(WordError::EmptySelection);
        }
        if req.tag_ids.is_empty() {
            return Err(WordError::EmptyTagSelection);
        }
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let tag_ids = Self::validated_tag_ids(db, book_id, &req.tag_ids).await?;
        let tagged = WordRepo::batch_tag(db, book_id, &req.word_ids, &tag_ids).await?;
        // 标签关系变化影响筛选后的打乱结果集合：洗牌缓存失效（词数不变，无需刷新 wordbooks 缓存）
        state.shuffle_cache.lock().clear();
        Ok(BatchTagWordsResp { tagged })
    }

    /// 解析 `tag` 查询参数（逗号分隔的标签 id）：排序去重；非法输入报错。
    pub(crate) fn parse_tag_ids(raw: Option<&str>) -> Result<Vec<i32>, WordError> {
        let Some(s) = raw else {
            return Ok(Vec::new());
        };
        if s.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut ids = Vec::new();
        for part in s.split(',') {
            let t = part.trim();
            if t.is_empty() {
                continue;
            }
            match t.parse::<i32>() {
                Ok(id) => ids.push(id),
                Err(_) => {
                    return Err(WordError::InvalidTagIds { tag: s.to_string() });
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    /// csv 模板：表头行，UTF-8 带 BOM（Excel/WPS 直接打开不乱码）。
    pub fn template_csv() -> Vec<u8> {
        let mut wtr = csv::Writer::from_writer(Vec::new());
        wtr.write_record(import::HEADERS).expect("写入表头");
        let mut buf = wtr.into_inner().expect("取回缓冲区");
        let mut out = Vec::with_capacity(buf.len() + 3);
        out.extend_from_slice(b"\xEF\xBB\xBF");
        out.append(&mut buf);
        out
    }

    /// xlsx 模板：Sheet1 表头（含样式与列宽），Sheet2 导入说明。
    pub fn template_xlsx() -> Result<Vec<u8>, WordError> {
        use rust_xlsxwriter::{Format, Workbook, Worksheet};
        let err = |e: rust_xlsxwriter::XlsxError| WordError::Template(e.to_string());
        let mut wb = Workbook::new();
        // Sheet1：表头
        let header = Format::new()
            .set_bold()
            .set_background_color("C58F6D")
            .set_font_color("FFFFFF");
        let mut sheet = Worksheet::new();
        sheet.set_name("单词模板").map_err(err)?;
        for (col, width) in [(0u16, 20.0), (1, 20.0), (2, 10.0), (3, 40.0), (4, 40.0), (5, 20.0)] {
            sheet.set_column_width(col, width).map_err(err)?;
        }
        sheet.set_row_height(0, 22.0).map_err(err)?;
        for (i, h) in import::HEADERS.iter().enumerate() {
            sheet
                .write_string_with_format(0, i as u16, *h, &header)
                .map_err(err)?;
        }
        wb.push_worksheet(sheet);
        // Sheet2：说明
        let mut info = Worksheet::new();
        info.set_name("说明").map_err(err)?;
        let lines = [
            "导入说明",
            "1. 第一行为表头，请勿修改；从第二行开始填写，每行一个义项（一个词性对应一个释义）。",
            "2. 同一单词的多个义项写多行（单词列重复填写），导入时自动合并为一个单词。",
            "3. 词性列填写该义项词性（如 n.），释义列填写对应释义；名词可分可数 C / 不可数 U / 两者 CU（小写 c / u / cu 同样接受），动词分及物 vt. / 不及物 vi.；完整可选：n. / c / C / u / U / cu / CU / v. / vt. / vi. / adj. / adv. / prep. / conj. / pron. / num. / art. / interj. / aux. / abbr. / phr.，可留空。",
            "4. 音标、例句、标签可留空；同一单词多行时音标/例句取首个非空，标签取并集。",
            "5. 标签列多个标签用分号（；）或英文分号（;）分隔；该书不存在的标签导入时自动创建。",
            "6. 保存为 .xlsx / .xls / .ods / .csv 后上传导入（WPS 请另存为 .xlsx 或 .csv）。",
            "7. 同书已存在相同拼写的单词：默认以模板内容更新该词并合并标签；可在导入预览中改为跳过。",
            "8. 上传后先预览：错误行可在预览中直接修改，确认后导入；有误的行会跳过并提示行号。",
        ];
        for (i, line) in lines.iter().enumerate() {
            info.write_string(i as u32, 0, *line).map_err(err)?;
        }
        wb.push_worksheet(info);
        wb.save_to_buffer().map_err(err)
    }

    async fn ensure_book_exists(db: &DatabaseConnection, book_id: i32) -> Result<(), WordError> {
        if WordRepo::find_wordbook(db, book_id).await?.is_none() {
            return Err(WordError::WordbookNotFound {
                wordbook_id: book_id,
            });
        }
        Ok(())
    }

    /// 供 router 层校验单词书存在（模板下载等场景）。
    pub(crate) async fn book_exists(state: &AppState, book_id: i32) -> Result<(), WordError> {
        Self::ensure_book_exists(state.db.as_ref(), book_id).await
    }

    fn to_resp(model: &word::Model, tags: &[i32]) -> Result<WordResp, WordError> {
        let definitions: Vec<Definition> = serde_json::from_value(model.definitions.clone())?;
        Ok(WordResp {
            id: model.id,
            wordbook_id: model.wordbook_id,
            spelling: model.spelling.clone(),
            phonetic: model.phonetic.clone(),
            definitions,
            example: model.example.clone(),
            tags: tags.to_vec(),
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
        })
    }

    fn to_page(
        models: Vec<word::Model>,
        tags_by_id: &HashMap<i32, Vec<i32>>,
        total: u64,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, WordError> {
        let items = models
            .iter()
            .map(|m| Self::to_resp(m, tags_by_id.get(&m.id).map_or(&[], |v| v)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PageResp {
            items,
            total,
            page,
            page_size,
            total_pages: total.div_ceil(page_size),
        })
    }

    /// 查询结果的标签数据加载 + 组装分页（一次查全部 id 的标签，避免 N+1）。
    async fn to_page_with_tags(
        db: &DatabaseConnection,
        models: Vec<word::Model>,
        total: u64,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, WordError> {
        let ids: Vec<i32> = models.iter().map(|m| m.id).collect();
        let tags_by_id = WordRepo::find_tag_ids_by_word_ids(db, &ids).await?;
        Self::to_page(models, &tags_by_id, total, page, page_size)
    }

    /// 校验标签 id 集合全部属于该书；返回去重升序后的 id（空集直接通过）。
    async fn validated_tag_ids(
        db: &DatabaseConnection,
        book_id: i32,
        ids: &[i32],
    ) -> Result<Vec<i32>, WordError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut unique = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let found = WordRepo::find_tag_ids_by_book(db, book_id, &unique).await?;
        if found.len() != unique.len() {
            let missing = unique
                .iter()
                .find(|id| !found.contains(id))
                .copied()
                .unwrap_or(0);
            return Err(WordError::TagNotInWordbook {
                tag_id: missing,
                wordbook_id: book_id,
            });
        }
        Ok(found)
    }

    /// 创建/更新前的字段校验：拼写、释义数量与词性白名单。
    pub(crate) fn validate(spelling: &str, definitions: &[Definition]) -> Result<(), WordError> {
        if spelling.trim().is_empty() {
            return Err(WordError::EmptySpelling);
        }
        if definitions.is_empty() {
            return Err(WordError::EmptyDefinitions);
        }
        if definitions.iter().any(|d| d.meaning.trim().is_empty()) {
            return Err(WordError::EmptyMeaning);
        }
        // 词性必须是合法枚举（与前端下拉一致）或为空
        const VALID_POS: [&str; 21] = [
            "n.", "c", "C", "u", "U", "cu", "CU", "v.", "vt.", "vi.", "adj.", "adv.",
            "prep.", "conj.", "pron.", "num.", "art.", "interj.", "aux.", "abbr.", "phr.",
        ];
        if let Some(bad) = definitions
            .iter()
            .find(|d| !d.pos.trim().is_empty() && !VALID_POS.contains(&d.pos.trim()))
        {
            return Err(WordError::InvalidPos {
                pos: bad.pos.clone(),
            });
        }
        Ok(())
    }

    /// 按 seed 确定性洗牌（Fisher-Yates + xorshift64*）：同一 seed 结果一致、跨数据库一致。
    fn seeded_shuffle(ids: &mut [i32], seed: &str) {
        // FNV-1a 把 seed 字符串映射为 u64 种子（置 1 避免 0）
        let mut s: u64 = 0xcbf29ce484222325;
        for b in seed.bytes() {
            s ^= u64::from(b);
            s = s.wrapping_mul(0x100000001b3);
        }
        s |= 1;
        for i in (1..ids.len()).rev() {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let j = (s % (i as u64 + 1)) as usize;
            ids.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use entity::definition::Definition;

    use super::WordService;
    use crate::business::wordbooks::words::error::WordError;

    fn definition(pos: &str, meaning: &str) -> Definition {
        Definition {
            pos: pos.into(),
            meaning: meaning.into(),
        }
    }

    #[test]
    fn validate_accepts_valid_word() {
        WordService::validate("apple", &[definition("n.", "苹果")]).unwrap();
    }

    #[test]
    fn validate_rejects_empty_spelling() {
        assert!(matches!(
            WordService::validate("  ", &[definition("", "苹果")]),
            Err(WordError::EmptySpelling)
        ));
    }

    #[test]
    fn validate_rejects_empty_definitions() {
        assert!(matches!(
            WordService::validate("apple", &[]),
            Err(WordError::EmptyDefinitions)
        ));
    }

    #[test]
    fn validate_rejects_empty_meaning() {
        assert!(matches!(
            WordService::validate("apple", &[definition("n.", "  ")]),
            Err(WordError::EmptyMeaning)
        ));
    }

    #[test]
    fn validate_rejects_unknown_pos() {
        assert!(matches!(
            WordService::validate("apple", &[definition("x.", "苹果")]),
            Err(WordError::InvalidPos { pos }) if pos == "x."
        ));
    }

    #[test]
    fn validate_allows_empty_pos() {
        WordService::validate("apple", &[definition("", "苹果")]).unwrap();
    }

    #[test]
    fn shuffle_is_deterministic() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut b = a.clone();
        WordService::seeded_shuffle(&mut a, "seed-1");
        WordService::seeded_shuffle(&mut b, "seed-1");
        assert_eq!(a, b);
    }

    #[test]
    fn shuffle_is_permutation() {
        let mut ids = (1..=100).collect::<Vec<_>>();
        WordService::seeded_shuffle(&mut ids, "hello");
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (1..=100).collect::<Vec<_>>());
    }

    #[test]
    fn shuffle_differs_by_seed() {
        let mut a = (1..=20).collect::<Vec<_>>();
        let mut b = a.clone();
        WordService::seeded_shuffle(&mut a, "seed-1");
        WordService::seeded_shuffle(&mut b, "seed-2");
        assert_ne!(a, b);
    }

    #[test]
    fn parse_tag_ids_dedups_and_sorts() {
        assert_eq!(
            WordService::parse_tag_ids(Some("3,1,2,2,1")).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            WordService::parse_tag_ids(Some("1,,2")).unwrap(),
            vec![1, 2]
        );
        assert_eq!(WordService::parse_tag_ids(Some(" 1 ")).unwrap(), vec![1]);
    }

    #[test]
    fn parse_tag_ids_handles_empty() {
        assert_eq!(WordService::parse_tag_ids(None).unwrap(), Vec::<i32>::new());
        assert_eq!(
            WordService::parse_tag_ids(Some("")).unwrap(),
            Vec::<i32>::new()
        );
        assert_eq!(
            WordService::parse_tag_ids(Some(" , ")).unwrap(),
            Vec::<i32>::new()
        );
    }

    #[test]
    fn parse_tag_ids_rejects_non_numeric() {
        assert!(matches!(
            WordService::parse_tag_ids(Some("1,abc")),
            Err(WordError::InvalidTagIds { tag }) if tag == "1,abc"
        ));
    }
}
