use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::task::JoinSet;

use super::cache_key::DictKey;
use super::cached_lookup::CachedLookup;
use super::dto::resp::{DictLookupResp, DictProviderErrorResp, DictProviderResp};
use super::entry::DictEntry;
use super::error::DictError;
use super::provider::DictProvider;
use super::provider_error::DictProviderError;
use super::provider_id::DictProviderId;
use super::provider_outcome::ProviderOutcome;
use super::state::DictState;

/// 词典业务逻辑：输入校验、源解析、缓存/单飞/熔断编排与结果聚合。
pub struct DictService;

impl DictService {
    /// 已知词典源列表（含是否启用），顺序固定为 `DictProviderId::ALL`。
    pub fn providers(dict: &DictState) -> Vec<DictProviderResp> {
        DictProviderId::ALL
            .iter()
            .map(|id| DictProviderResp {
                id: *id,
                label: id.label().to_string(),
                enabled: dict.config.providers.contains(id),
            })
            .collect()
    }

    /// 聚合抓词：并发请求各源，单源失败只进 `errors`，不影响其它源。
    pub async fn lookup(
        dict: &Arc<DictState>,
        word: Option<&str>,
        providers: Option<&str>,
        refresh: bool,
    ) -> Result<DictLookupResp, DictError> {
        if !dict.enabled() {
            return Err(DictError::DictDisabled);
        }
        let word = validate_word(word.unwrap_or(""), dict.config.max_word_len)?;
        let ids = resolve_ids(dict, providers)?;

        let mut set: JoinSet<ProviderOutcome> = JoinSet::new();
        for id in ids.iter().copied() {
            let dict = Arc::clone(dict);
            let word = word.clone();
            set.spawn(async move { lookup_one(&dict, id, &word, refresh).await });
        }
        let mut outcomes: HashMap<DictProviderId, ProviderOutcome> = HashMap::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(outcome) => {
                    outcomes.insert(outcome.provider, outcome);
                }
                Err(e) => tracing::error!("词典抓取任务异常: {e}"),
            }
        }

        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for id in ids {
            match outcomes.remove(&id) {
                Some(outcome) => match outcome.result {
                    Ok(entry) => entries.push(entry),
                    Err(err) => {
                        tracing::warn!("词典源 {} 抓取失败: {err}", id.label());
                        errors.push(DictProviderErrorResp {
                            provider: id,
                            label: id.label().to_string(),
                            message: err.user_message().to_string(),
                        });
                    }
                },
                None => errors.push(DictProviderErrorResp {
                    provider: id,
                    label: id.label().to_string(),
                    message: "词典抓取任务异常".to_string(),
                }),
            }
        }
        Ok(DictLookupResp {
            word,
            entries,
            errors,
        })
    }
}

/// 单个源的完整流程：缓存 → 熔断 → 并发闸门 → 单飞 → 再查缓存 → 请求上游。
async fn lookup_one(
    dict: &Arc<DictState>,
    id: DictProviderId,
    word: &str,
    refresh: bool,
) -> ProviderOutcome {
    let Some(provider) = dict.provider(id).cloned() else {
        return ProviderOutcome {
            provider: id,
            result: Err(DictProviderError::Parse("词典源未注册".into())),
        };
    };
    let key = DictKey::new(id, word);
    if !refresh {
        if let Some(hit) = dict.cache.get(&key, Instant::now()) {
            return ProviderOutcome {
                provider: id,
                result: cached_result(hit),
            };
        }
    }
    if !dict.breakers.allow(id, Instant::now()) {
        return ProviderOutcome {
            provider: id,
            result: Err(DictProviderError::Unavailable),
        };
    }

    let _permit = dict.semaphore.clone().acquire_owned().await.ok();
    let gate = dict.singleflight.gate(&key);
    let result = {
        let _lock = gate.lock().await;
        let cached = if refresh {
            None
        } else {
            dict.cache.get(&key, Instant::now())
        };
        match cached {
            Some(hit) => cached_result(hit),
            None => fetch_and_store(dict, provider.as_ref(), id, &key, word).await,
        }
    };
    dict.singleflight.release(&key);
    ProviderOutcome {
        provider: id,
        result,
    }
}

/// 请求上游并按结果写缓存/更新熔断状态。
async fn fetch_and_store(
    dict: &DictState,
    provider: &dyn DictProvider,
    id: DictProviderId,
    key: &DictKey,
    word: &str,
) -> Result<DictEntry, DictProviderError> {
    let limit = Duration::from_millis(dict.config.timeout_ms.saturating_add(1000));
    match tokio::time::timeout(limit, provider.lookup(&dict.http, word)).await {
        Ok(Ok(entry)) => {
            dict.cache.insert(
                key.clone(),
                CachedLookup::Found(Arc::new(entry.clone())),
                Instant::now(),
            );
            dict.breakers.on_success(id);
            Ok(entry)
        }
        // 上游正常响应但未收录：写负缓存，不算故障
        Ok(Err(DictProviderError::NotFound)) => {
            dict.cache
                .insert(key.clone(), CachedLookup::NotFound, Instant::now());
            dict.breakers.on_success(id);
            Err(DictProviderError::NotFound)
        }
        Ok(Err(err)) => {
            dict.breakers.on_failure(id, Instant::now());
            Err(err)
        }
        Err(_) => {
            dict.breakers.on_failure(id, Instant::now());
            Err(DictProviderError::Timeout)
        }
    }
}

fn cached_result(hit: CachedLookup) -> Result<DictEntry, DictProviderError> {
    match hit {
        CachedLookup::Found(entry) => Ok((*entry).clone()),
        CachedLookup::NotFound => Err(DictProviderError::NotFound),
    }
}

/// 校验并规范化单词：去首尾空白、长度上限、允许字符白名单。
pub(super) fn validate_word(raw: &str, max: usize) -> Result<String, DictError> {
    let word = raw.trim();
    if word.is_empty() {
        return Err(DictError::EmptyWord);
    }
    if word.chars().count() > max {
        return Err(DictError::WordTooLong { max });
    }
    // 允许拉丁扩展字母（café / naïve）与连字符、撇号、空格、句点
    let allowed = |c: char| {
        c.is_alphabetic() || c.is_ascii_digit() || matches!(c, ' ' | '-' | '\'' | '\u{2019}' | '.')
    };
    if !word.chars().all(allowed) {
        return Err(DictError::InvalidWord);
    }
    Ok(word.to_string())
}

/// 解析查询参数里的源列表：缺省 = 配置启用源；未知源/未启用源均报错。
fn resolve_ids(dict: &DictState, raw: Option<&str>) -> Result<Vec<DictProviderId>, DictError> {
    let raw = raw.map(str::trim).unwrap_or("");
    if raw.is_empty() {
        return Ok(dict.config.providers.clone());
    }
    let mut ids: Vec<DictProviderId> = Vec::new();
    for token in raw
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let id = DictProviderId::from_str(token).map_err(|_| DictError::UnknownProvider {
            provider: token.to_string(),
        })?;
        if !dict.config.providers.contains(&id) {
            return Err(DictError::ProviderDisabled {
                provider: token.to_string(),
            });
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return Ok(dict.config.providers.clone());
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;

    use super::DictService;
    use crate::business::dict::entry::DictEntry;
    use crate::business::dict::error::DictError;
    use crate::business::dict::http::DictHttp;
    use crate::business::dict::provider::DictProvider;
    use crate::business::dict::provider_error::DictProviderError;
    use crate::business::dict::provider_id::DictProviderId;
    use crate::business::dict::state::DictState;
    use crate::config::dict::DictConfig;

    /// 可编程的假词典源：统计调用次数，可注入延迟与结果。
    struct MockProvider {
        id: DictProviderId,
        calls: Arc<AtomicUsize>,
        delay: Duration,
        outcome: MockOutcome,
    }

    #[derive(Clone)]
    enum MockOutcome {
        Ok,
        NotFound,
        Fail,
    }

    #[async_trait]
    impl DictProvider for MockProvider {
        fn id(&self) -> DictProviderId {
            self.id
        }

        fn label(&self) -> &'static str {
            self.id.label()
        }

        async fn lookup(
            &self,
            _http: &DictHttp,
            word: &str,
        ) -> Result<DictEntry, DictProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if !self.delay.is_zero() {
                tokio::time::sleep(self.delay).await;
            }
            match self.outcome {
                MockOutcome::Ok => Ok(entry(word)),
                MockOutcome::NotFound => Err(DictProviderError::NotFound),
                MockOutcome::Fail => Err(DictProviderError::Http("boom".into())),
            }
        }
    }

    fn entry(word: &str) -> DictEntry {
        DictEntry {
            provider: DictProviderId::Youdao,
            label: "有道词典".into(),
            word: word.to_string(),
            phonetic: Some("/əˈbændən/".into()),
            definitions: Vec::new(),
            examples: Vec::new(),
            audio_url: None,
            source_url: None,
        }
    }

    fn mock(
        id: DictProviderId,
        calls: &Arc<AtomicUsize>,
        delay: Duration,
        outcome: MockOutcome,
    ) -> Arc<dyn DictProvider> {
        Arc::new(MockProvider {
            id,
            calls: Arc::clone(calls),
            delay,
            outcome,
        })
    }

    fn state_with(
        providers: Vec<Arc<dyn DictProvider>>,
        ids: Vec<DictProviderId>,
    ) -> Arc<DictState> {
        let config = DictConfig {
            providers: ids,
            ..DictConfig::default()
        };
        let mut state = DictState::new(&config);
        state.providers = providers;
        Arc::new(state)
    }

    fn counter() -> Arc<AtomicUsize> {
        Arc::new(AtomicUsize::new(0))
    }

    #[test]
    fn lists_all_providers_with_enabled_flag() {
        let state = state_with(
            Vec::new(),
            vec![DictProviderId::Youdao, DictProviderId::Urban],
        );
        let list = DictService::providers(&state);
        assert_eq!(list.len(), 8);
        assert_eq!(list[0].id, DictProviderId::Youdao);
        assert_eq!(list[0].label, "有道词典");
        assert!(list[0].enabled);
        assert!(!list[1].enabled);
        assert!(list[7].enabled);
    }

    #[tokio::test]
    async fn rejects_invalid_words() {
        let state = state_with(Vec::new(), vec![DictProviderId::Youdao]);
        assert_eq!(
            DictService::lookup(&state, Some("   "), None, false)
                .await
                .unwrap_err(),
            DictError::EmptyWord
        );
        assert_eq!(
            DictService::lookup(&state, None, None, false)
                .await
                .unwrap_err(),
            DictError::EmptyWord
        );
        // 字母（含中文/重音字母）合法，标点符号非法
        for invalid in ["abandon!", "放弃?", "a@b", "foo_bar", "<script>"] {
            assert_eq!(
                DictService::lookup(&state, Some(invalid), None, false)
                    .await
                    .unwrap_err(),
                DictError::InvalidWord,
                "应拒绝非法单词: {invalid}"
            );
        }
        let too_long = "a".repeat(65);
        assert_eq!(
            DictService::lookup(&state, Some(&too_long), None, false)
                .await
                .unwrap_err(),
            DictError::WordTooLong { max: 64 }
        );
    }

    #[tokio::test]
    async fn accepts_accented_words() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::ZERO,
                MockOutcome::Ok,
            )],
            vec![DictProviderId::Youdao],
        );
        for word in ["café", "naïve", "Über", "rock-n-roll", "o'clock", "e.g."] {
            let resp = DictService::lookup(&state, Some(word), None, false)
                .await
                .unwrap();
            assert_eq!(resp.word, word);
        }
    }

    #[tokio::test]
    async fn rejects_unknown_and_disabled_providers() {
        let state = state_with(Vec::new(), vec![DictProviderId::Youdao]);
        assert_eq!(
            DictService::lookup(&state, Some("abandon"), Some("foo"), false)
                .await
                .unwrap_err(),
            DictError::UnknownProvider {
                provider: "foo".into()
            }
        );
        assert_eq!(
            DictService::lookup(&state, Some("abandon"), Some("youdao,urban"), false)
                .await
                .unwrap_err(),
            DictError::ProviderDisabled {
                provider: "urban".into()
            }
        );
    }

    #[tokio::test]
    async fn rejects_when_dict_disabled() {
        let config = DictConfig {
            enabled: false,
            ..DictConfig::default()
        };
        let state = Arc::new(DictState::new(&config));
        assert_eq!(
            DictService::lookup(&state, Some("abandon"), None, false)
                .await
                .unwrap_err(),
            DictError::DictDisabled
        );
    }

    #[tokio::test]
    async fn single_failure_does_not_break_other_providers() {
        let ok_calls = counter();
        let bad_calls = counter();
        let state = state_with(
            vec![
                mock(
                    DictProviderId::Youdao,
                    &ok_calls,
                    Duration::ZERO,
                    MockOutcome::Ok,
                ),
                mock(
                    DictProviderId::Baidu,
                    &bad_calls,
                    Duration::ZERO,
                    MockOutcome::Fail,
                ),
            ],
            vec![DictProviderId::Youdao, DictProviderId::Baidu],
        );
        let resp = DictService::lookup(&state, Some("abandon"), None, false)
            .await
            .unwrap();
        assert_eq!(resp.word, "abandon");
        assert_eq!(resp.entries.len(), 1);
        assert_eq!(resp.entries[0].provider, DictProviderId::Youdao);
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(resp.errors[0].provider, DictProviderId::Baidu);
        assert_eq!(resp.errors[0].label, "百度翻译");
        assert_eq!(resp.errors[0].message, "网络请求失败");
        assert_eq!(ok_calls.load(Ordering::SeqCst), 1);
        assert_eq!(bad_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cache_hit_skips_second_upstream_call() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::ZERO,
                MockOutcome::Ok,
            )],
            vec![DictProviderId::Youdao],
        );
        let first = DictService::lookup(&state, Some("abandon"), None, false)
            .await
            .unwrap();
        let second = DictService::lookup(&state, Some("Abandon"), None, false)
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "缓存命中不应再请求上游");
        assert_eq!(first.entries.len(), 1);
        assert_eq!(second.entries.len(), 1);
    }

    #[tokio::test]
    async fn refresh_bypasses_cache() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::ZERO,
                MockOutcome::Ok,
            )],
            vec![DictProviderId::Youdao],
        );
        for _ in 0..2 {
            DictService::lookup(&state, Some("abandon"), None, true)
                .await
                .unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2, "refresh 应绕过缓存");
    }

    #[tokio::test]
    async fn not_found_is_negatively_cached() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::ZERO,
                MockOutcome::NotFound,
            )],
            vec![DictProviderId::Youdao],
        );
        for _ in 0..2 {
            let resp = DictService::lookup(&state, Some("abandon"), None, false)
                .await
                .unwrap();
            assert!(resp.entries.is_empty());
            assert_eq!(resp.errors.len(), 1);
            assert_eq!(resp.errors[0].message, "未找到该单词的释义");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "未收录结果应写负缓存");
    }

    #[tokio::test]
    async fn concurrent_identical_lookups_hit_upstream_once() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::from_millis(50),
                MockOutcome::Ok,
            )],
            vec![DictProviderId::Youdao],
        );
        let (first, second) = tokio::join!(
            DictService::lookup(&state, Some("abandon"), None, false),
            DictService::lookup(&state, Some("abandon"), None, false),
        );
        assert_eq!(first.unwrap().entries.len(), 1);
        assert_eq!(second.unwrap().entries.len(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "单飞应合并并发请求");
    }

    #[tokio::test]
    async fn breaker_short_circuits_after_repeated_failures() {
        let calls = counter();
        let state = state_with(
            vec![mock(
                DictProviderId::Youdao,
                &calls,
                Duration::ZERO,
                MockOutcome::Fail,
            )],
            vec![DictProviderId::Youdao],
        );
        for _ in 0..3 {
            let resp = DictService::lookup(&state, Some("abandon"), None, true)
                .await
                .unwrap();
            assert_eq!(resp.errors[0].message, "网络请求失败");
        }
        let resp = DictService::lookup(&state, Some("abandon"), None, true)
            .await
            .unwrap();
        assert!(resp.entries.is_empty());
        assert_eq!(resp.errors[0].message, "词典源暂时不可用，请稍后再试");
        assert_eq!(calls.load(Ordering::SeqCst), 3, "熔断后不应再请求上游");
    }
}
