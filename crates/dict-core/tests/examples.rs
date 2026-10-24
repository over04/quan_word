use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use dict_core::engine_limits::EngineLimits;
use dict_core::entry::DictEntry;
use dict_core::plugin_source::PluginSource;
use dict_core::provider_error::DictProviderError;
use dict_core::provider_id::ProviderId;
use dict_core::search_engine::SearchEngine;
use dict_core::search_request::SearchRequest;
use dict_core::search_response::SearchResponse;
use dict_core::transport::SearchTransport;
use dict_core::transport_error::TransportError;

struct FixtureTransport {
    by_host: HashMap<&'static str, &'static str>,
    suggest: &'static str,
}

#[async_trait]
impl SearchTransport for FixtureTransport {
    async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError> {
        if request.url.contains("dict.youdao.com/suggest") {
            return Ok(SearchResponse {
                status: 200,
                body: self.suggest.to_string(),
                final_url: request.url,
            });
        }
        for (host, body) in &self.by_host {
            if request.url.contains(host) {
                return Ok(SearchResponse {
                    status: 200,
                    body: (*body).to_string(),
                    final_url: request.url,
                });
            }
        }
        Err(TransportError::NotFound)
    }
}

struct SingleTransport(&'static str);

#[async_trait]
impl SearchTransport for SingleTransport {
    async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError> {
        Ok(SearchResponse {
            status: 200,
            body: self.0.to_string(),
            final_url: request.url,
        })
    }
}

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn example_sources() -> Vec<PluginSource> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(examples_dir().join("plugins"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            PluginSource::new(file, std::fs::read(path).unwrap())
        })
        .collect()
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(examples_dir().join("fixtures").join(name)).unwrap()
}

fn leaked_fixture(name: &str) -> &'static str {
    Box::leak(fixture(name).into_boxed_str())
}

async fn lookup_with(id: &str, body: &'static str) -> Result<DictEntry, DictProviderError> {
    let engine = SearchEngine::new(Arc::new(SingleTransport(body)), EngineLimits::default());
    engine.load(example_sources());
    engine
        .lookup(&ProviderId::new(id).unwrap(), "abandon")
        .await
}

fn golden(name: &str) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(examples_dir().join("golden").join(format!("{name}.json")))
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn fallback_branches_match_legacy_goldens() {
    let cases = [
        ("bing", "bing_network.html", "bing_network"),
        ("bing", "bing_list.html", "bing_list"),
        ("baidu", "baidu_fallback.json", "baidu_fallback"),
    ];
    for (id, fixture_name, golden_name) in cases {
        let body = leaked_fixture(fixture_name);
        let entry = lookup_with(id, body)
            .await
            .unwrap_or_else(|error| panic!("{golden_name}: {error}"));
        assert_eq!(
            serde_json::to_value(&entry).unwrap(),
            golden(golden_name),
            "{golden_name}"
        );
    }
}

#[tokio::test]
async fn empty_payloads_map_to_expected_errors() {
    assert_eq!(
        lookup_with("youdao", leaked_fixture("youdao_empty.json"))
            .await
            .unwrap_err(),
        DictProviderError::NotFound
    );
    assert!(matches!(
        lookup_with("eudic", leaked_fixture("eudic_empty.html"))
            .await
            .unwrap_err(),
        DictProviderError::Parse(_)
    ));
    for (id, body) in [
        ("iciba", r#"{"message":[]}"#),
        ("free_dict", "[]"),
        ("wiktionary", "{}"),
        ("urban", r#"{"list":[]}"#),
    ] {
        assert_eq!(
            lookup_with(id, body).await.unwrap_err(),
            DictProviderError::NotFound,
            "{id}"
        );
    }
}

#[tokio::test]
async fn suggest_dedups_case_insensitively() {
    let engine = SearchEngine::new(Arc::new(transport()), EngineLimits::default());
    engine.load(example_sources());
    let suggestions = engine
        .suggest("abandon", &[ProviderId::new("youdao").unwrap()])
        .await;
    assert_eq!(
        serde_json::to_value(&suggestions).unwrap(),
        serde_json::json!([
            {"word": "abandon", "summary": "v. 放弃"},
            {"word": "abandoned", "summary": "adj. 被抛弃的"}
        ])
    );
}

fn transport() -> FixtureTransport {
    FixtureTransport {
        by_host: HashMap::from([
            ("dict.youdao.com/jsonapi", leaked_fixture("youdao.json")),
            ("fanyi.baidu.com/sug", leaked_fixture("baidu.json")),
            ("dict.eudic.net/minidicts", leaked_fixture("eudic.html")),
            ("cn.bing.com/dict/search", leaked_fixture("bing.html")),
            (
                "dict.iciba.com/dictionary/word/suggestion",
                leaked_fixture("iciba.json"),
            ),
            (
                "api.dictionaryapi.dev/api/v2/entries",
                leaked_fixture("free_dict.json"),
            ),
            (
                "en.wiktionary.org/api/rest_v1",
                leaked_fixture("wiktionary.json"),
            ),
            (
                "api.urbandictionary.com/v0/define",
                leaked_fixture("urban.json"),
            ),
        ]),
        suggest: leaked_fixture("suggest_youdao.json"),
    }
}

#[tokio::test]
async fn examples_match_legacy_goldens() {
    let engine = SearchEngine::new(Arc::new(transport()), EngineLimits::default());
    let report = engine.load(example_sources());
    assert_eq!(report.loaded, 8, "reports: {:?}", report.reports);
    for id in [
        "youdao",
        "baidu",
        "eudic",
        "bing",
        "iciba",
        "free_dict",
        "wiktionary",
        "urban",
    ] {
        let entry = engine
            .lookup(&ProviderId::new(id).unwrap(), "abandon")
            .await
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        let actual = serde_json::to_value(&entry).unwrap();
        assert_eq!(actual, golden(id), "{id} 与旧解析器输出不一致");
    }
}

#[tokio::test]
async fn examples_parse_primary_fixtures() {
    let engine = SearchEngine::new(Arc::new(transport()), EngineLimits::default());
    let report = engine.load(example_sources());
    assert_eq!(report.loaded, 8, "reports: {:?}", report.reports);
    for id in [
        "youdao",
        "baidu",
        "eudic",
        "bing",
        "iciba",
        "free_dict",
        "wiktionary",
        "urban",
    ] {
        let entry = engine
            .lookup(&ProviderId::new(id).unwrap(), "abandon")
            .await
            .unwrap_or_else(|error| panic!("{id}: {error}"));
        println!(
            "===== {id} =====
{}",
            serde_json::to_string_pretty(&entry).unwrap()
        );
    }
    let suggestions = engine
        .suggest("abandon", &[ProviderId::new("youdao").unwrap()])
        .await;
    println!(
        "===== suggest =====
{}",
        serde_json::to_string_pretty(&suggestions).unwrap()
    );
}
