use std::collections::HashMap;

use chrono::Utc;
use entity::{word, wordbook};
use sea_orm::{
    sea_query::{Condition, Expr, ExprTrait}, ActiveModelTrait, ColumnTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

use crate::dto::req::create_word_req::CreateWordReq;
use crate::dto::req::update_word_req::UpdateWordReq;
use crate::dto::resp::page_resp::PageResp;
use crate::dto::resp::word_resp::WordResp;
use crate::error::ApiError;
use crate::model::definition::Definition;

/// 单词列表排序方式：纸质书浏览顺序（id / 字母 / seeded 随机打乱）。
pub enum WordOrder {
    IdAsc,
    IdDesc,
    Spelling,
    Random(String),
}

impl WordOrder {
    /// 解析 order / seed 查询参数；白名单外的值返回 400。
    pub fn parse(order: Option<&str>, seed: Option<&str>) -> Result<Self, ApiError> {
        match order {
            None | Some("id_asc") => Ok(Self::IdAsc),
            Some("id_desc") => Ok(Self::IdDesc),
            Some("spelling") => Ok(Self::Spelling),
            Some("random") => match seed.filter(|s| !s.is_empty()) {
                Some(seed) => Ok(Self::Random(seed.to_string())),
                None => Err(ApiError::BadRequest("order=random 需要 seed 参数".into())),
            },
            Some(other) => Err(ApiError::BadRequest(format!("不支持的排序: {other}"))),
        }
    }
}

/// 单词业务逻辑：分页查询（含排序/打乱）/ 搜索查询 / 创建 / 更新 / 删除。
pub struct WordService;

impl WordService {
    pub async fn list(
        db: &DatabaseConnection,
        book_id: i32,
        page: u64,
        page_size: u64,
        order: &WordOrder,
    ) -> Result<PageResp<WordResp>, ApiError> {
        Self::ensure_book_exists(db, book_id).await?;
        match order {
            // SQL 层排序分页：id / 字母序
            WordOrder::IdAsc | WordOrder::IdDesc | WordOrder::Spelling => {
                let mut q = word::Entity::find().filter(word::Column::WordbookId.eq(book_id));
                q = match order {
                    WordOrder::IdAsc => q.order_by_asc(word::Column::Id),
                    WordOrder::IdDesc => q.order_by_desc(word::Column::Id),
                    _ => q.order_by_asc(word::Column::Spelling),
                };
                let paginator = q.paginate(db, page_size);
                let models = paginator.fetch_page(page - 1).await?;
                let total = paginator.num_items().await?;
                Self::to_page(models, total, page, page_size)
            }
            // 打乱：全量 id 按 seed 确定性洗牌（跨库一致），按页取 id 切片后查单词
            WordOrder::Random(seed) => {
                let ids: Vec<i32> = word::Entity::find()
                    .select_only()
                    .column(word::Column::Id)
                    .filter(word::Column::WordbookId.eq(book_id))
                    .into_tuple()
                    .all(db)
                    .await?;
                let mut ordered = ids;
                seeded_shuffle(&mut ordered, seed);
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
                    word::Entity::find()
                        .filter(word::Column::WordbookId.eq(book_id))
                        .filter(word::Column::Id.is_in(slice.iter().copied()))
                        .all(db)
                        .await?
                };
                // `IN` 查询结果无序：按切片顺序重排
                let by_id: HashMap<i32, word::Model> =
                    models.into_iter().map(|m| (m.id, m)).collect();
                let ordered_models: Vec<word::Model> = slice
                    .iter()
                    .filter_map(|id| by_id.get(id).cloned())
                    .collect();
                Self::to_page(ordered_models, total, page, page_size)
            }
        }
    }

    /// 列表模式查询：书内搜索（拼写/释义模糊匹配）+ 白名单排序 + 分页。
    pub async fn query(
        db: &DatabaseConnection,
        book_id: i32,
        q: Option<String>,
        sort: &str,
        order: &str,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, ApiError> {
        Self::ensure_book_exists(db, book_id).await?;
        let mut query = word::Entity::find().filter(word::Column::WordbookId.eq(book_id));
        if let Some(q) = q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let pat = format!("%{q}%");
            query = query.filter(
                Condition::any()
                    .add(word::Column::Spelling.like(&pat))
                    .add(word::Column::Phonetic.like(&pat))
                    .add(word::Column::Example.like(&pat))
                    .add(Expr::cust("CAST(definitions AS TEXT)").like(&pat)),
            );
        }
        let column = match sort {
            "spelling" => word::Column::Spelling,
            "created_at" => word::Column::CreatedAt,
            "updated_at" => word::Column::UpdatedAt,
            other => return Err(ApiError::BadRequest(format!("不支持的排序字段: {other}"))),
        };
        query = match order {
            "asc" => query.order_by_asc(column),
            "desc" => query.order_by_desc(column),
            other => return Err(ApiError::BadRequest(format!("order 必须为 asc 或 desc: {other}"))),
        };
        let paginator = query.paginate(db, page_size);
        let models = paginator.fetch_page(page - 1).await?;
        let total = paginator.num_items().await?;
        Self::to_page(models, total, page, page_size)
    }

    pub async fn create(
        db: &DatabaseConnection,
        book_id: i32,
        req: CreateWordReq,
    ) -> Result<WordResp, ApiError> {
        validate(&req.spelling, &req.definitions)?;
        Self::ensure_book_exists(db, book_id).await?;
        let now = Utc::now();
        let model = word::ActiveModel {
            wordbook_id: Set(book_id),
            spelling: Set(req.spelling),
            phonetic: Set(req.phonetic),
            definitions: Set(serde_json::to_value(&req.definitions).map_err(to_internal)?),
            example: Set(req.example),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Self::to_resp(&model)
    }

    pub async fn update(
        db: &DatabaseConnection,
        id: i32,
        req: UpdateWordReq,
    ) -> Result<WordResp, ApiError> {
        validate(&req.spelling, &req.definitions)?;
        let w = word::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("单词 {id} 不存在")))?;
        let mut model: word::ActiveModel = w.into();
        model.spelling = Set(req.spelling);
        model.phonetic = Set(req.phonetic);
        model.definitions = Set(serde_json::to_value(&req.definitions).map_err(to_internal)?);
        model.example = Set(req.example);
        model.updated_at = Set(Utc::now());
        let saved = model.update(db).await?;
        Self::to_resp(&saved)
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), ApiError> {
        let res = word::Entity::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(ApiError::NotFound(format!("单词 {id} 不存在")));
        }
        Ok(())
    }

    async fn ensure_book_exists(db: &DatabaseConnection, book_id: i32) -> Result<(), ApiError> {
        let exists = wordbook::Entity::find_by_id(book_id).one(db).await?;
        if exists.is_none() {
            return Err(ApiError::NotFound(format!("单词书 {book_id} 不存在")));
        }
        Ok(())
    }

    fn to_resp(model: &word::Model) -> Result<WordResp, ApiError> {
        let definitions: Vec<Definition> =
            serde_json::from_value(model.definitions.clone()).map_err(to_internal)?;
        Ok(WordResp {
            id: model.id,
            wordbook_id: model.wordbook_id,
            spelling: model.spelling.clone(),
            phonetic: model.phonetic.clone(),
            definitions,
            example: model.example.clone(),
        })
    }

    fn to_page(
        models: Vec<word::Model>,
        total: u64,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, ApiError> {
        let items = models
            .iter()
            .map(Self::to_resp)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PageResp {
            items,
            total,
            page,
            page_size,
            total_pages: total.div_ceil(page_size),
        })
    }
}

fn validate(spelling: &str, definitions: &[Definition]) -> Result<(), ApiError> {
    if spelling.trim().is_empty() {
        return Err(ApiError::BadRequest("单词不能为空".into()));
    }
    if definitions.is_empty() {
        return Err(ApiError::BadRequest("至少需要一个释义".into()));
    }
    if definitions.iter().any(|d| d.meaning.trim().is_empty()) {
        return Err(ApiError::BadRequest("释义内容不能为空".into()));
    }
    // 词性必须是合法枚举（与前端下拉一致）或为空
    const VALID_POS: [&str; 13] = [
        "n.", "v.", "adj.", "adv.", "prep.", "conj.", "pron.", "num.", "art.",
        "interj.", "aux.", "abbr.", "phr.",
    ];
    if definitions.iter().any(|d| !d.pos.trim().is_empty() && !VALID_POS.contains(&d.pos.trim())) {
        return Err(ApiError::BadRequest(format!(
            "词性不合法: {}，可选：{} 或留空",
            definitions
                .iter()
                .map(|d| d.pos.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            VALID_POS.join(" / ")
        )));
    }
    Ok(())
}

fn to_internal(e: serde_json::Error) -> ApiError {
    ApiError::Internal(anyhow::anyhow!("释义数据格式错误: {e}"))
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
