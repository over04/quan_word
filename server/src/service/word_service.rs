use chrono::Utc;
use entity::{word, wordbook};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

use crate::dto::req::create_word_req::CreateWordReq;
use crate::dto::req::update_word_req::UpdateWordReq;
use crate::dto::resp::page_resp::PageResp;
use crate::dto::resp::word_resp::WordResp;
use crate::error::ApiError;
use crate::model::definition::Definition;

/// 单词业务逻辑：分页查询 / 创建 / 更新 / 删除。
pub struct WordService;

impl WordService {
    pub async fn list(
        db: &DatabaseConnection,
        book_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, ApiError> {
        Self::ensure_book_exists(db, book_id).await?;
        let paginator = word::Entity::find()
            .filter(word::Column::WordbookId.eq(book_id))
            .order_by_asc(word::Column::Id)
            .paginate(db, page_size);
        let models = paginator.fetch_page(page - 1).await?;
        let total = paginator.num_items().await?;
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
