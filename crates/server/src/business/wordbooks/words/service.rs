use std::collections::HashMap;

use chrono::Utc;
use entity::definition::Definition;
use entity::word;
use sea_orm::{DatabaseConnection, Set};

use super::dto::create::CreateWordReq;
use super::dto::resp::WordResp;
use super::dto::update::UpdateWordReq;
use super::error::WordError;
use super::order::WordOrder;
use super::repo::WordRepo;
use super::sort::SortField;
use super::sort_dir::SortDir;
use crate::common::model::page::PageResp;
use crate::common::state::{AppState, SHUFFLE_CACHE_CAP};

/// 单词业务逻辑：分页查询（含排序/打乱）/ 搜索查询 / 创建 / 更新 / 删除。
pub struct WordService;

impl WordService {
    pub async fn list(
        state: &AppState,
        book_id: i32,
        page: u64,
        page_size: u64,
        order: &WordOrder,
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
                )
                .await?;
                Self::to_page(models, total, page, page_size)
            }
            WordOrder::IdDesc => {
                let (models, total) = WordRepo::browse_page(
                    db,
                    book_id,
                    word::Column::Id,
                    SortDir::Desc,
                    page,
                    page_size,
                )
                .await?;
                Self::to_page(models, total, page, page_size)
            }
            WordOrder::Spelling => {
                let (models, total) = WordRepo::browse_page(
                    db,
                    book_id,
                    word::Column::Spelling,
                    SortDir::Asc,
                    page,
                    page_size,
                )
                .await?;
                Self::to_page(models, total, page, page_size)
            }
            // 打乱：全量 id 按 seed 确定性洗牌（跨库一致），按页取 id 切片后查单词
            WordOrder::Random(seed) => {
                // 洗牌序列缓存：(book_id, seed) → 完整 id 序列，避免每页请求重复全量洗牌
                // 注意：锁 guard 立即 drop，禁止跨 await 持有（parking_lot guard 非 Send）
                let key = (book_id, seed.clone());
                let cached = state.shuffle_cache.lock().get(&key).cloned();
                let ordered = match cached {
                    Some(v) => v,
                    None => {
                        let mut ordered = WordRepo::find_all_ids(db, book_id).await?;
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
                Self::to_page(ordered_models, total, page, page_size)
            }
        }
    }

    /// 列表模式查询：书内搜索（拼写/释义模糊匹配）+ 白名单排序 + 分页。
    pub async fn query(
        state: &AppState,
        book_id: i32,
        q: Option<String>,
        field: SortField,
        dir: SortDir,
        page: u64,
        page_size: u64,
    ) -> Result<PageResp<WordResp>, WordError> {
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let q = q.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let (models, total) =
            WordRepo::search_page(db, book_id, q, field, dir, page, page_size).await?;
        Self::to_page(models, total, page, page_size)
    }

    pub async fn create(
        state: &AppState,
        book_id: i32,
        req: CreateWordReq,
    ) -> Result<WordResp, WordError> {
        Self::validate(&req.spelling, &req.definitions)?;
        let db = state.db.as_ref();
        Self::ensure_book_exists(db, book_id).await?;
        let now = Utc::now();
        let model = WordRepo::insert(
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
        )
        .await?;
        // 新词影响列表页 word_count 与洗牌 id 集合：两个缓存都失效
        state.invalidate_wordbooks();
        state.shuffle_cache.lock().clear();
        Self::to_resp(&model)
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
        let mut model: word::ActiveModel = w.into();
        model.spelling = Set(req.spelling);
        model.phonetic = Set(req.phonetic);
        model.definitions = Set(serde_json::to_value(&req.definitions)?);
        model.example = Set(req.example);
        model.updated_at = Set(Utc::now());
        let saved = WordRepo::update(db, model).await?;
        Self::to_resp(&saved)
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

    async fn ensure_book_exists(db: &DatabaseConnection, book_id: i32) -> Result<(), WordError> {
        if WordRepo::find_wordbook(db, book_id).await?.is_none() {
            return Err(WordError::WordbookNotFound {
                wordbook_id: book_id,
            });
        }
        Ok(())
    }

    fn to_resp(model: &word::Model) -> Result<WordResp, WordError> {
        let definitions: Vec<Definition> = serde_json::from_value(model.definitions.clone())?;
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
    ) -> Result<PageResp<WordResp>, WordError> {
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

    /// 创建/更新前的字段校验：拼写、释义数量与词性白名单。
    fn validate(spelling: &str, definitions: &[Definition]) -> Result<(), WordError> {
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
        const VALID_POS: [&str; 13] = [
            "n.", "v.", "adj.", "adv.", "prep.", "conj.", "pron.", "num.", "art.", "interj.",
            "aux.", "abbr.", "phr.",
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
}
