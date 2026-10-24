use std::time::Instant;

use reqwest::Url;

use super::dto::suggest_resp::DictSuggestResp;
use super::dto::suggestion::DictSuggestion;
use super::error::DictError;
use super::http::NO_HEADERS;
use super::service::validate_word;
use super::state::DictState;
use super::suggest_wire::SuggestWire;

/// 有道 suggest 接口地址（模糊匹配 + 简短释义）。
const SUGGEST_URL: &str = "https://dict.youdao.com/suggest";
/// 单次最多返回的建议条数。
/// 主页结果列表按 6 条一批展开（前端 DictResultList 的 PAGE），8 条翻不动，给到 24 条够三轮。
/// 有道 suggest 的 num 参数实测支持 24（q=wha 返回 24 条）。
const SUGGEST_LIMIT: usize = 24;

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
        let suggestions = fetch(&dict.http, &word).await;
        dict.suggest_cache
            .insert(key, suggestions.clone(), Instant::now());
        Ok(DictSuggestResp { word, suggestions })
    }
}

/// 拼上游请求 URL。
///
/// 单独成函数是为了能单测 `num` 与 `SUGGEST_LIMIT` 始终一致、`q` 被正确编码
/// （以前 `num` 内联在 `fetch` 里，改常量时没有任何断言兜底）。
fn suggest_url(word: &str) -> Option<Url> {
    let mut url = Url::parse(SUGGEST_URL).ok()?;
    url.query_pairs_mut()
        .append_pair("num", &SUGGEST_LIMIT.to_string())
        .append_pair("ver", "3.0")
        .append_pair("doctype", "json")
        .append_pair("cache", "false")
        .append_pair("le", "en")
        .append_pair("q", word);
    Some(url)
}

/// 请求上游并解析；上游失败返回空列表（模糊建议不应阻塞详情查询）。
async fn fetch(http: &super::http::DictHttp, word: &str) -> Vec<DictSuggestion> {
    let Some(url) = suggest_url(word) else {
        return Vec::new();
    };
    let Ok(body) = http.get_text(url.as_str(), &NO_HEADERS).await else {
        return Vec::new();
    };
    let Ok(wire) = serde_json::from_str::<SuggestWire>(&body) else {
        return Vec::new();
    };
    let Some(payload) = wire.data else {
        return Vec::new();
    };
    let mut seen: Vec<String> = Vec::new();
    let mut suggestions = Vec::new();
    for item in payload.entries {
        let entry = item.entry.trim().to_string();
        let summary = item.explain.trim().to_string();
        if entry.is_empty() || summary.is_empty() {
            continue;
        }
        let lower = entry.to_lowercase();
        if seen.contains(&lower) {
            continue;
        }
        seen.push(lower);
        suggestions.push(DictSuggestion {
            word: entry,
            summary,
        });
        if suggestions.len() >= SUGGEST_LIMIT {
            break;
        }
    }
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `num` 必须跟随 `SUGGEST_LIMIT`：改常量却忘了改请求参数时这条会红。
    #[test]
    fn url_carries_suggest_limit() {
        let url = suggest_url("game").expect("url");
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        // 字面量锁死：常量被改小/改大时这条会红，提醒同步前端 PAGE 的假设
        assert_eq!(pairs.get("num").map(String::as_str), Some("24"));
        assert_eq!(SUGGEST_LIMIT, 24);
        assert_eq!(pairs.get("q").map(String::as_str), Some("game"));
    }

    /// 查询词必须被编码：空格 / `&` / 中文都不能截断参数。
    #[test]
    fn url_encodes_query_word() {
        let url = suggest_url("a b&c 中文").expect("url");
        let q = url
            .query_pairs()
            .find(|(key, _)| key == "q")
            .map(|(_, value)| value.into_owned())
            .expect("q");
        assert_eq!(q, "a b&c 中文");
        assert!(url.as_str().contains("q=a+b%26c"));
    }
}
