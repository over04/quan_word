use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dict_core::engine_limits::EngineLimits;
use dict_core::entry::DictEntry;
use dict_core::plugin_info::PluginInfo;
use dict_core::plugin_package::PluginPackage;
use dict_core::plugin_source::PluginSource;
use dict_core::provider_error::DictProviderError;
use dict_core::provider_id::ProviderId;
use dict_core::recipe::dedup_mode::DedupMode;
use dict_core::recipe::definition_strategy::DefinitionStrategy;
use dict_core::recipe::empty_behavior::EmptyBehavior;
use dict_core::recipe::lookup_recipe::LookupRecipe;
use dict_core::recipe::request_spec::RequestSpec;
use dict_core::recipe::suggest_recipe::SuggestRecipe;
use dict_core::registry_entry::RegistryEntry;
use tokio::task::JoinSet;

use super::cache_key::DictKey;
use super::cached_lookup::CachedLookup;
use super::dto::plugin_detail::field::DictPluginFieldResp;
use super::dto::plugin_detail::field_key::PluginFieldKey;
use super::dto::plugin_detail::limits::DictPluginLimitsResp;
use super::dto::plugin_detail::lookup::DictPluginLookupResp;
use super::dto::plugin_detail::request::DictPluginRequestResp;
use super::dto::plugin_detail::resp::DictPluginDetailResp;
use super::dto::plugin_detail::strategy::DictPluginStrategyResp;
use super::dto::plugin_detail::strategy_kind::PluginStrategyKind;
use super::dto::plugin_detail::suggest::DictPluginSuggestResp;
use super::dto::resp::{DictLookupResp, DictProviderErrorResp, DictProviderResp};
use super::error::DictError;
use super::provider_outcome::ProviderOutcome;
use super::state::DictState;

/// 词典业务逻辑：输入校验、源解析、缓存/单飞/熔断编排与结果聚合。
pub struct DictService;

impl DictService {
    /// 已安装插件列表（含是否启用），顺序 = 注册表顺序（文件名排序）。
    pub fn providers(dict: &DictState) -> Vec<DictProviderResp> {
        dict.engine
            .plugins()
            .into_iter()
            .map(|plugin| provider_resp(dict, plugin))
            .collect()
    }

    /// 单个插件的详情蓝图（设置页可视化）；未知 id 返回 404。
    pub fn plugin_detail(
        dict: &DictState,
        id: &ProviderId,
    ) -> Result<DictPluginDetailResp, DictError> {
        let entry = dict
            .engine
            .entry(id)
            .ok_or_else(|| DictError::PluginNotFound { id: id.to_string() })?;
        Ok(detail_resp(
            &entry,
            dict.is_enabled(id),
            dict.engine.limits(),
        ))
    }

    /// 运行时启停单个插件；偏好落盘，重启后保持。
    pub fn set_enabled(
        dict: &DictState,
        id: &ProviderId,
        enabled: bool,
    ) -> Result<DictProviderResp, DictError> {
        let plugin = dict
            .set_enabled(id, enabled)
            .ok_or_else(|| DictError::UnknownProvider {
                provider: id.to_string(),
            })?;
        Ok(provider_resp(dict, plugin))
    }

    /// 导入插件包：校验后写入插件目录，覆盖同 id 的旧外部包，并立即重载。
    pub async fn import_plugin(
        dict: &Arc<DictState>,
        file_name: &str,
        bytes: Vec<u8>,
    ) -> Result<Vec<DictProviderResp>, DictError> {
        let source = PluginSource::new(file_name, bytes);
        let package = PluginPackage::parse(&source, dict.engine.limits())
            .map_err(|error| DictError::InvalidPlugin(error.to_string()))?;
        let id = package.manifest.id.clone();
        let installed = dict.engine.plugins();
        if !installed.iter().any(|plugin| plugin.id == id)
            && installed.len() >= dict.config.max_plugins
        {
            return Err(DictError::InvalidPlugin("插件数量已达上限".into()));
        }
        remove_external_by_id(dict, &id)?;
        dict.plugin_store
            .write_external(&id, &source.bytes)
            .map_err(|error| DictError::PluginIo(error.to_string()))?;
        dict.reload_plugins().await;
        if dict.plugin(&id).is_none() {
            return Err(DictError::InvalidPlugin("插件装载失败".into()));
        }
        Ok(Self::providers(dict))
    }

    /// 删除外部插件文件并立即重载；内置已不存在，未知 id 直接报错。
    pub async fn delete_plugin(
        dict: &Arc<DictState>,
        id: &ProviderId,
    ) -> Result<Vec<DictProviderResp>, DictError> {
        if dict.plugin(id).is_none() {
            return Err(DictError::UnknownProvider {
                provider: id.to_string(),
            });
        }
        if remove_external_by_id(dict, id)? == 0 {
            return Err(DictError::UnknownProvider {
                provider: id.to_string(),
            });
        }
        dict.reload_plugins().await;
        Ok(Self::providers(dict))
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
        for id in ids.iter().cloned() {
            let dict = Arc::clone(dict);
            let word = word.clone();
            set.spawn(async move { lookup_one(&dict, id, &word, refresh).await });
        }
        let mut outcomes: HashMap<ProviderId, ProviderOutcome> = HashMap::new();
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(outcome) => {
                    outcomes.insert(outcome.provider.clone(), outcome);
                }
                Err(error) => tracing::error!("词典抓取任务异常: {error}"),
            }
        }

        let mut entries = Vec::new();
        let mut errors = Vec::new();
        for id in ids {
            let label = dict
                .plugin(&id)
                .map_or_else(|| id.to_string(), |plugin| plugin.label);
            match outcomes.remove(&id) {
                Some(outcome) => match outcome.result {
                    Ok(entry) => entries.push(entry),
                    Err(error) => {
                        tracing::warn!("词典源 {label} 抓取失败: {error}");
                        errors.push(DictProviderErrorResp {
                            provider: id,
                            label,
                            message: error.user_message().to_string(),
                        });
                    }
                },
                None => errors.push(DictProviderErrorResp {
                    provider: id,
                    label,
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
    id: ProviderId,
    word: &str,
    refresh: bool,
) -> ProviderOutcome {
    let Some(plugin) = dict.plugin(&id) else {
        return ProviderOutcome {
            provider: id,
            result: Err(DictProviderError::Parse("词典源未注册".into())),
        };
    };
    let key = DictKey::new(id.clone(), &plugin.version, word);
    if !refresh {
        if let Some(hit) = dict.cache.get(&key, Instant::now()) {
            return ProviderOutcome {
                provider: id,
                result: cached_result(hit),
            };
        }
    }
    if !dict.breakers.allow(&id, Instant::now()) {
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
            None => fetch_and_store(dict, &id, &key, word).await,
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
    id: &ProviderId,
    key: &DictKey,
    word: &str,
) -> Result<DictEntry, DictProviderError> {
    let limit = Duration::from_millis(dict.config.timeout_ms.saturating_add(1000));
    match tokio::time::timeout(limit, dict.engine.lookup(id, word)).await {
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
        Ok(Err(error)) => {
            dict.breakers.on_failure(id, Instant::now());
            Err(error)
        }
        Err(_) => {
            dict.breakers.on_failure(id, Instant::now());
            Err(DictProviderError::Timeout)
        }
    }
}

/// 删除所有解析后 id 匹配的外部插件文件；返回删除数量。
fn remove_external_by_id(dict: &DictState, id: &ProviderId) -> Result<usize, DictError> {
    let mut removed = 0;
    for path in dict.plugin_store.external_paths() {
        let bytes = fs::read(&path).map_err(|error| DictError::PluginIo(error.to_string()))?;
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        let source = PluginSource::new(file, bytes);
        let Ok(package) = PluginPackage::parse(&source, dict.engine.limits()) else {
            continue;
        };
        if package.manifest.id == *id {
            fs::remove_file(&path).map_err(|error| DictError::PluginIo(error.to_string()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// 把注册表条目压成前端可视化需要的蓝图；只暴露字段名/数量，不暴露请求头或表单值。
fn detail_resp(
    entry: &RegistryEntry,
    enabled: bool,
    ceilings: &EngineLimits,
) -> DictPluginDetailResp {
    let lookup_request = entry
        .recipe
        .lookup
        .as_ref()
        .map(|lookup| request_resp(&lookup.request));
    let lookup = entry.recipe.lookup.as_ref().map(lookup_resp);
    let suggest_request = entry
        .recipe
        .suggest
        .as_ref()
        .map(|suggest| request_resp(&suggest.request));
    let suggest = entry.recipe.suggest.as_ref().map(suggest_resp);
    DictPluginDetailResp {
        id: entry.info.id.clone(),
        label: entry.info.label.clone(),
        short_label: entry.info.short_label.clone(),
        description: entry.info.description.clone(),
        version: entry.info.version.clone(),
        enabled,
        capabilities: entry.info.capabilities.clone(),
        hosts: entry.info.hosts.clone(),
        lookup_request,
        lookup,
        suggest_request,
        suggest,
        limits: DictPluginLimitsResp {
            max_definitions: entry.limits.max_definitions,
            max_examples: entry.limits.max_examples,
            max_body_kib: entry.limits.max_body_bytes / 1024,
            ceiling_definitions: ceilings.max_definitions,
            ceiling_examples: ceilings.max_examples,
            ceiling_body_kib: ceilings.max_body_bytes / 1024,
        },
    }
}

fn request_resp(request: &RequestSpec) -> DictPluginRequestResp {
    DictPluginRequestResp {
        method: request.method,
        path: request_path(&request.url),
        headers: request
            .headers
            .iter()
            .map(|header| header.name.clone())
            .collect(),
        form: request
            .form
            .iter()
            .map(|field| field.name.clone())
            .collect(),
    }
}

fn request_path(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = rest.find('/').map_or("/", |index| &rest[index..]);
    path.split('?')
        .next()
        .unwrap_or("/")
        .replace("{word}", ":word")
}

fn lookup_resp(lookup: &LookupRecipe) -> DictPluginLookupResp {
    let mut fields = Vec::new();
    if lookup.word.is_some() {
        fields.push(field(PluginFieldKey::Word, 1, Vec::new()));
    }
    if lookup.phonetic.is_some() {
        fields.push(field(PluginFieldKey::Phonetic, 1, Vec::new()));
    }
    if lookup.audio_url.is_some() {
        fields.push(field(PluginFieldKey::Audio, 1, Vec::new()));
    }
    if lookup.source_url.is_some() {
        fields.push(field(PluginFieldKey::Source, 1, Vec::new()));
    }
    if !lookup.definitions.is_empty() {
        fields.push(field(
            PluginFieldKey::Definitions,
            lookup.definitions.len(),
            Vec::new(),
        ));
    }
    if let Some(examples) = &lookup.examples {
        let names: Vec<String> = examples.fields.keys().cloned().collect();
        fields.push(field(PluginFieldKey::Examples, names.len().max(1), names));
    }
    DictPluginLookupResp {
        fields,
        strategies: lookup.definitions.iter().map(strategy_resp).collect(),
        vars: lookup.vars.iter().map(|var| var.name.clone()).collect(),
        empty_is_parse: lookup.empty == EmptyBehavior::Parse,
    }
}

fn field(key: PluginFieldKey, count: usize, names: Vec<String>) -> DictPluginFieldResp {
    DictPluginFieldResp { key, count, names }
}

fn strategy_resp(strategy: &DefinitionStrategy) -> DictPluginStrategyResp {
    match strategy {
        DefinitionStrategy::PosText {
            when, transform, ..
        } => strategy_value(
            PluginStrategyKind::PosText,
            false,
            when.is_some(),
            transform.len(),
        ),
        DefinitionStrategy::PlainText {
            when, transform, ..
        } => strategy_value(
            PluginStrategyKind::PlainText,
            false,
            when.is_some(),
            transform.len(),
        ),
        DefinitionStrategy::Items {
            pos,
            when,
            transform,
            ..
        } => strategy_value(
            PluginStrategyKind::Items,
            pos.is_some(),
            when.is_some(),
            transform.len(),
        ),
        DefinitionStrategy::Groups {
            pos,
            when,
            transform,
            ..
        } => strategy_value(
            PluginStrategyKind::Groups,
            pos.is_some(),
            when.is_some(),
            transform.len(),
        ),
    }
}

fn strategy_value(
    kind: PluginStrategyKind,
    has_pos: bool,
    has_condition: bool,
    transforms: usize,
) -> DictPluginStrategyResp {
    DictPluginStrategyResp {
        kind,
        has_pos,
        has_condition,
        transforms,
    }
}

fn suggest_resp(suggest: &SuggestRecipe) -> DictPluginSuggestResp {
    DictPluginSuggestResp {
        fields: suggest.items.fields.keys().cloned().collect(),
        take: suggest.take,
        dedup: suggest.dedup.map(|mode| match mode {
            DedupMode::WordLowercase => "word_lowercase".to_string(),
        }),
    }
}

fn provider_resp(dict: &DictState, plugin: PluginInfo) -> DictProviderResp {
    let enabled = dict.is_enabled(&plugin.id);
    DictProviderResp {
        id: plugin.id,
        label: plugin.label,
        short_label: plugin.short_label,
        description: plugin.description,
        version: plugin.version,
        capabilities: plugin.capabilities,
        enabled,
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

/// 解析查询参数里的源列表：缺省 = 配置启用源；未知/未启用源均报错。
fn resolve_ids(dict: &DictState, raw: Option<&str>) -> Result<Vec<ProviderId>, DictError> {
    let raw = raw.map(str::trim).unwrap_or("");
    if raw.is_empty() {
        return non_empty(dict.enabled_ids());
    }
    let mut ids: Vec<ProviderId> = Vec::new();
    for token in raw
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let id = ProviderId::new(token).map_err(|_| DictError::UnknownProvider {
            provider: token.to_string(),
        })?;
        if dict.plugin(&id).is_none() {
            return Err(DictError::UnknownProvider {
                provider: token.to_string(),
            });
        }
        if !dict.is_enabled(&id) {
            return Err(DictError::ProviderDisabled {
                provider: token.to_string(),
            });
        }
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        return non_empty(dict.enabled_ids());
    }
    Ok(ids)
}

fn non_empty(ids: Vec<ProviderId>) -> Result<Vec<ProviderId>, DictError> {
    if ids.is_empty() {
        Err(DictError::NoProviders)
    } else {
        Ok(ids)
    }
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use dict_core::engine_limits::EngineLimits;
    use dict_core::plugin_source::PluginSource;
    use dict_core::provider_id::ProviderId;
    use dict_core::request_method::RequestMethod;
    use dict_core::search_engine::SearchEngine;
    use dict_core::search_request::SearchRequest;
    use dict_core::search_response::SearchResponse;
    use dict_core::transport::SearchTransport;
    use dict_core::transport_error::TransportError;

    use super::DictService;
    use crate::business::dict::dto::plugin_detail::field_key::PluginFieldKey;
    use crate::business::dict::error::DictError;
    use crate::business::dict::state::DictState;
    use crate::config::dict::DictConfig;

    struct MockResponse {
        delay: Duration,
        result: Result<&'static str, TransportError>,
    }

    type Handler = Arc<dyn Fn(&str) -> MockResponse + Send + Sync>;

    struct MockTransport {
        handler: Handler,
    }

    #[async_trait]
    impl SearchTransport for MockTransport {
        async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError> {
            let response = (self.handler)(&request.url);
            if !response.delay.is_zero() {
                tokio::time::sleep(response.delay).await;
            }
            match response.result {
                Ok(body) => Ok(SearchResponse {
                    status: 200,
                    body: body.to_string(),
                    final_url: request.url,
                }),
                Err(error) => Err(error),
            }
        }
    }

    fn plugin_source(id: &str) -> PluginSource {
        let body = format!(
            r#"{{"manifest":{{"api_version":1,"id":"{id}","label":"{id}","version":"1.0.0","engine":"recipe","capabilities":["lookup"],"hosts":["mock.local"]}},"recipe":{{"lookup":{{"request":{{"method":"get","url":"https://mock.local/{id}?q={{word}}"}},"definitions":[{{"kind":"plain_text","from":{{"path":"definition"}},"separators":[";"]}}]}}}}}}"#
        );
        PluginSource::new(format!("{id}.json"), body.into_bytes())
    }

    fn engine(handler: Handler) -> Arc<SearchEngine> {
        let engine =
            SearchEngine::new(Arc::new(MockTransport { handler }), EngineLimits::default());
        let report = engine.load(vec![plugin_source("youdao"), plugin_source("baidu")]);
        assert_eq!(report.loaded, 2, "reports: {:?}", report.reports);
        Arc::new(engine)
    }

    fn state(engine: Arc<SearchEngine>, providers: &[&str]) -> Arc<DictState> {
        let config = DictConfig {
            providers: providers.iter().map(|id| (*id).to_string()).collect(),
            ..DictConfig::default()
        };
        Arc::new(DictState::from_engine(&config, engine).unwrap())
    }

    fn ok_handler(calls: &Arc<AtomicUsize>) -> Handler {
        let calls = Arc::clone(calls);
        Arc::new(move |_url| {
            calls.fetch_add(1, Ordering::SeqCst);
            MockResponse {
                delay: Duration::ZERO,
                result: Ok(r#"{"definition":"放弃;抛弃"}"#),
            }
        })
    }

    fn id(raw: &str) -> ProviderId {
        ProviderId::new(raw).unwrap()
    }

    #[test]
    fn builds_plugin_detail_blueprint() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        let detail = DictService::plugin_detail(&dict, &id("youdao")).unwrap();
        assert!(detail.enabled);
        assert_eq!(detail.hosts, vec!["mock.local"]);
        let request = detail.lookup_request.as_ref().unwrap();
        assert_eq!(request.method, RequestMethod::Get);
        assert_eq!(request.path, "/youdao");
        let lookup = detail.lookup.as_ref().unwrap();
        assert_eq!(lookup.strategies.len(), 1);
        assert_eq!(lookup.fields.len(), 1);
        assert_eq!(lookup.fields[0].key, PluginFieldKey::Definitions);
        assert_eq!(lookup.fields[0].count, 1);
        assert_eq!(detail.limits.max_definitions, 20);
        assert_eq!(detail.limits.ceiling_definitions, 20);
        assert_eq!(
            DictService::plugin_detail(&dict, &id("missing")).unwrap_err(),
            DictError::PluginNotFound {
                id: "missing".into()
            }
        );
    }

    #[test]
    fn lists_installed_plugins_with_enabled_flag() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        let list = DictService::providers(&dict);
        assert_eq!(list.len(), 2);
        let youdao = list.iter().find(|item| item.id == id("youdao")).unwrap();
        assert!(youdao.enabled);
        assert_eq!(youdao.short_label, "youdao");
        assert_eq!(youdao.capabilities.len(), 1);
        let baidu = list.iter().find(|item| item.id == id("baidu")).unwrap();
        assert!(!baidu.enabled, "未在配置里的源不启用");
    }

    #[tokio::test]
    async fn rejects_invalid_words() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        assert_eq!(
            DictService::lookup(&dict, Some("   "), None, false)
                .await
                .unwrap_err(),
            DictError::EmptyWord
        );
        assert_eq!(
            DictService::lookup(&dict, None, None, false)
                .await
                .unwrap_err(),
            DictError::EmptyWord
        );
        for invalid in ["abandon!", "放弃?", "a@b", "foo_bar", "<script>"] {
            assert_eq!(
                DictService::lookup(&dict, Some(invalid), None, false)
                    .await
                    .unwrap_err(),
                DictError::InvalidWord,
                "应拒绝非法单词: {invalid}"
            );
        }
        let too_long = "a".repeat(65);
        assert_eq!(
            DictService::lookup(&dict, Some(&too_long), None, false)
                .await
                .unwrap_err(),
            DictError::WordTooLong { max: 64 }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0, "输入非法时不应请求上游");
    }

    #[tokio::test]
    async fn accepts_accented_words() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        for word in ["café", "naïve", "Über", "rock-n-roll", "o'clock", "e.g."] {
            let resp = DictService::lookup(&dict, Some(word), None, false)
                .await
                .unwrap();
            assert_eq!(resp.word, word);
            assert_eq!(resp.entries.len(), 1);
        }
    }

    #[tokio::test]
    async fn rejects_unknown_and_disabled_providers() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        assert_eq!(
            DictService::lookup(&dict, Some("abandon"), Some("foo"), false)
                .await
                .unwrap_err(),
            DictError::UnknownProvider {
                provider: "foo".into()
            }
        );
        assert_eq!(
            DictService::lookup(&dict, Some("abandon"), Some("youdao,baidu"), false)
                .await
                .unwrap_err(),
            DictError::ProviderDisabled {
                provider: "baidu".into()
            }
        );
    }

    #[tokio::test]
    async fn rejects_when_dict_disabled() {
        let calls = Arc::new(AtomicUsize::new(0));
        let config = DictConfig {
            enabled: false,
            ..DictConfig::default()
        };
        let dict = Arc::new(DictState::from_engine(&config, engine(ok_handler(&calls))).unwrap());
        assert_eq!(
            DictService::lookup(&dict, Some("abandon"), None, false)
                .await
                .unwrap_err(),
            DictError::DictDisabled
        );
    }

    #[tokio::test]
    async fn single_failure_does_not_break_other_providers() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let handler: Handler = Arc::new(move |url| {
            counter.fetch_add(1, Ordering::SeqCst);
            if url.contains("/baidu") {
                MockResponse {
                    delay: Duration::ZERO,
                    result: Err(TransportError::Http("boom".into())),
                }
            } else {
                MockResponse {
                    delay: Duration::ZERO,
                    result: Ok(r#"{"definition":"放弃"}"#),
                }
            }
        });
        let dict = state(engine(handler), &["youdao", "baidu"]);
        let resp = DictService::lookup(&dict, Some("abandon"), None, false)
            .await
            .unwrap();
        assert_eq!(resp.entries.len(), 1);
        assert_eq!(resp.entries[0].provider, id("youdao"));
        assert_eq!(resp.errors.len(), 1);
        assert_eq!(resp.errors[0].provider, id("baidu"));
        assert_eq!(resp.errors[0].label, "baidu");
        assert_eq!(resp.errors[0].message, "网络请求失败");
    }

    #[tokio::test]
    async fn cache_hit_skips_second_upstream_call() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        let first = DictService::lookup(&dict, Some("abandon"), None, false)
            .await
            .unwrap();
        let second = DictService::lookup(&dict, Some("Abandon"), None, false)
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "缓存命中不应再请求上游");
        assert_eq!(first.entries.len(), 1);
        assert_eq!(second.entries.len(), 1);
    }

    #[tokio::test]
    async fn refresh_bypasses_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        for _ in 0..2 {
            DictService::lookup(&dict, Some("abandon"), None, true)
                .await
                .unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2, "refresh 应绕过缓存");
    }

    #[tokio::test]
    async fn not_found_is_negatively_cached() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let handler: Handler = Arc::new(move |_url| {
            counter.fetch_add(1, Ordering::SeqCst);
            MockResponse {
                delay: Duration::ZERO,
                result: Err(TransportError::NotFound),
            }
        });
        let dict = state(engine(handler), &["youdao"]);
        for _ in 0..2 {
            let resp = DictService::lookup(&dict, Some("abandon"), None, false)
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
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let handler: Handler = Arc::new(move |_url| {
            counter.fetch_add(1, Ordering::SeqCst);
            MockResponse {
                delay: Duration::from_millis(50),
                result: Ok(r#"{"definition":"放弃"}"#),
            }
        });
        let dict = state(engine(handler), &["youdao"]);
        let (first, second) = tokio::join!(
            DictService::lookup(&dict, Some("abandon"), None, false),
            DictService::lookup(&dict, Some("abandon"), None, false),
        );
        assert_eq!(first.unwrap().entries.len(), 1);
        assert_eq!(second.unwrap().entries.len(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "单飞应合并并发请求");
    }

    #[tokio::test]
    async fn breaker_short_circuits_after_repeated_failures() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let handler: Handler = Arc::new(move |_url| {
            counter.fetch_add(1, Ordering::SeqCst);
            MockResponse {
                delay: Duration::ZERO,
                result: Err(TransportError::Http("boom".into())),
            }
        });
        let dict = state(engine(handler), &["youdao"]);
        for _ in 0..3 {
            let resp = DictService::lookup(&dict, Some("abandon"), None, true)
                .await
                .unwrap();
            assert_eq!(resp.errors[0].message, "网络请求失败");
        }
        let resp = DictService::lookup(&dict, Some("abandon"), None, true)
            .await
            .unwrap();
        assert!(resp.entries.is_empty());
        assert_eq!(resp.errors[0].message, "词典源暂时不可用，请稍后再试");
        assert_eq!(calls.load(Ordering::SeqCst), 3, "熔断后不应再请求上游");
    }

    #[test]
    fn toggles_plugins_at_runtime() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dict = state(engine(ok_handler(&calls)), &["youdao"]);
        assert!(!dict.is_enabled(&id("baidu")));

        let enabled = DictService::set_enabled(&dict, &id("baidu"), true).unwrap();
        assert!(enabled.enabled);
        assert!(dict.is_enabled(&id("baidu")));
        assert_eq!(dict.enabled_ids(), vec![id("youdao"), id("baidu")]);

        let disabled = DictService::set_enabled(&dict, &id("youdao"), false).unwrap();
        assert!(!disabled.enabled);
        assert_eq!(dict.enabled_ids(), vec![id("baidu")]);

        assert_eq!(
            DictService::set_enabled(&dict, &id("missing"), true).unwrap_err(),
            DictError::UnknownProvider {
                provider: "missing".into()
            }
        );
    }
}
