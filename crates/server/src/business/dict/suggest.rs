use std::time::Instant;

use super::dto::suggest_resp::DictSuggestResp;
use super::error::DictError;
use super::service::validate_word;
use super::state::DictState;

/// 模糊查词建议：只做轻量列表，详情由详情接口提供。
pub struct SuggestService;

impl SuggestService {
    pub async fn suggest(
        dict: &DictState,
        word: Option<&str>,
    ) -> Result<DictSuggestResp, DictError> {
        if !dict.enabled() {
            return Err(DictError::DictDisabled);
        }
        let word = validate_word(word.unwrap_or(""), dict.config.max_word_len)?;
        let key = word.to_lowercase();
        let now = Instant::now();
        if let Some(hit) = dict.suggest_cache.get(&key, now) {
            return Ok(DictSuggestResp {
                word,
                suggestions: hit,
            });
        }
        let ids = dict.enabled_ids();
        let suggestions = if ids.is_empty() {
            Vec::new()
        } else {
            dict.engine.suggest(&word, &ids).await
        };
        dict.suggest_cache
            .insert(key, suggestions.clone(), Instant::now());
        Ok(DictSuggestResp { word, suggestions })
    }
}
