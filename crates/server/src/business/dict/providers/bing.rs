use std::sync::LazyLock;

use async_trait::async_trait;
use entity::definition::Definition;
use scraper::{Html, Selector};

use crate::business::dict::entry::DictEntry;
use crate::business::dict::http::DictHttp;
use crate::business::dict::http_header::HttpHeader;
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// 必应词典（`cn.bing.com/dict/search`，HTML 页面）。
pub struct BingProvider;

/// 必应按 Accept-Language 返回中文释义页。
const HEADERS: [HttpHeader; 1] = [HttpHeader::new("Accept-Language", "zh-CN")];

/// meta description 里「网络释义」之后是网络义项（无词性，噪声大）。
const NETWORK_MARKER: &str = "网络释义";

/// 必应 description 的固定句式「必应词典为您提供{word}的释义」；
/// 没有该标记说明不是词条页（或结构变了），不拿它当释义来源。
const DESCRIPTION_MARKER: &str = "的释义";

#[async_trait]
impl DictProvider for BingProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Bing
    }

    fn label(&self) -> &'static str {
        DictProviderId::Bing.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://cn.bing.com/dict/search?q={}&mkt=zh-cn",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &HEADERS).await?;
        parse_bing(&body, word)
    }
}

/// 编译一次的选择器（进程内复用）。
static META_DESCRIPTION_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| selector("meta[name=\"description\"]"));
static LIST_ITEM_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("li"));
static POS_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("span.pos"));
static DEFINITION_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("span.def"));

fn selector(raw: &str) -> Selector {
    Selector::parse(raw).expect("必应词典 CSS 选择器应合法")
}

/// 解析必应词典页。
///
/// 释义优先取 `meta[name=description]`（含完整主释义，li 只覆盖前几组）；
/// 主释义为空才回退「网络释义」段，再回退列表项。音标取美音（缺省英音）。
pub fn parse_bing(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let document = Html::parse_document(body);
    let description = document
        .select(&META_DESCRIPTION_SELECTOR)
        .next()
        .and_then(|meta| meta.value().attr("content"))
        .map(text::collapse_ws)
        .unwrap_or_default();

    let phonetic = text::between(&description, "美[", "]")
        .or_else(|| text::between(&description, "英[", "]"))
        .and_then(text::normalize_phonetic);

    let mut definitions = description_definitions(&description);
    if definitions.is_empty() {
        definitions = list_definitions(&document);
    }
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::Bing,
        label: DictProviderId::Bing.label().to_string(),
        word: word.to_string(),
        phonetic,
        definitions,
        examples: Vec::new(),
        audio_url: None,
        source_url: None,
    })
}

/// 从 meta description 取释义：先主释义，为空再退网络释义（词性留空）。
fn description_definitions(description: &str) -> Vec<Definition> {
    if !description.contains(DESCRIPTION_MARKER) {
        return Vec::new();
    }
    let (main, network) = description
        .split_once(NETWORK_MARKER)
        .unwrap_or((description, ""));
    let mut definitions = pos::pos_definitions(description_body(main), &['；', ';']);
    if definitions.is_empty() {
        for piece in description_body(network).split(['；', ';']) {
            pos::add_definition(&mut definitions, "", piece);
        }
    }
    definitions
}

/// 去掉「必应词典为您提供…释义，美[..]，英[..]，」前缀，只留释义正文。
fn description_body(description: &str) -> &str {
    let tail = description
        .rsplit_once(']')
        .map_or(description, |(_, tail)| tail);
    tail.trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, '，' | ',' | '：' | ':' | '；' | ';')
    })
}

/// 兜底：页面列表项里的「词性 + 释义」。
fn list_definitions(document: &Html) -> Vec<Definition> {
    let mut definitions = Vec::new();
    for item in document.select(&LIST_ITEM_SELECTOR) {
        let Some(pos_element) = item.select(&POS_SELECTOR).next() else {
            continue;
        };
        let Some(definition_element) = item.select(&DEFINITION_SELECTOR).next() else {
            continue;
        };
        let pos = pos::canonical_pos(&text::element_text(pos_element)).unwrap_or("");
        let meaning = text::element_text(definition_element);
        for piece in meaning.split([';', '；']) {
            pos::add_definition(&mut definitions, pos, piece);
        }
    }
    definitions
}

#[cfg(test)]
mod tests {
    use super::parse_bing;

    /// 真实页面片段（abandon）：meta description 携带完整主释义 + 网络释义。
    const FIXTURE: &str = r#"<html><head>
      <meta name="description" content="必应词典为您提供abandon的释义，美[əˈbændən]，英[ə'bændən]，v. 放弃；抛弃；废弃；遗弃； n. 放肆；狂热；任性；恣意任性； 网络释义： 丢弃；舍弃；离弃； " />
    </head><body>
      <ul>
        <li><span class="pos">v.</span><span class="def b_regtxt"><a href="/dict/search?q=放弃">放弃</a><span>；</span><a href="/dict/search?q=抛弃">抛弃</a></span></li>
        <li><span class="pos">n.</span><span class="def b_regtxt"><a href="/dict/search?q=放肆">放肆</a></span></li>
        <li><a href="/dict/search?q=abandon">无关条目</a></li>
      </ul>
    </body></html>"#;

    #[test]
    fn parses_full_definitions_from_meta_description() {
        let entry = parse_bing(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Bing);
        assert_eq!(entry.label, "必应词典");
        assert_eq!(entry.phonetic.as_deref(), Some("/əˈbændən/"));
        // 主释义 4 + 4，网络释义默认丢弃
        assert_eq!(entry.definitions.len(), 8);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert_eq!(entry.definitions[0].meaning, "放弃");
        assert_eq!(entry.definitions[3].meaning, "遗弃");
        assert_eq!(entry.definitions[4].pos, "n.");
        assert_eq!(entry.definitions[4].meaning, "放肆");
        assert_eq!(entry.definitions[7].meaning, "恣意任性");
        assert!(
            entry
                .definitions
                .iter()
                .all(|definition| !definition.meaning.contains("丢弃")),
            "网络释义默认丢弃"
        );
    }

    #[test]
    fn falls_back_to_network_section_when_main_is_empty() {
        let body = r#"<html><head>
          <meta name="description" content="必应词典为您提供abandon的释义，美[əˈbændən]，英[əˈbændən]，网络释义： 丢弃；舍弃" />
        </head><body></body></html>"#;
        let entry = parse_bing(body, "abandon").unwrap();
        assert_eq!(entry.definitions.len(), 2);
        assert_eq!(entry.definitions[0].pos, "");
        assert_eq!(entry.definitions[0].meaning, "丢弃");
    }

    #[test]
    fn falls_back_to_list_items_without_description() {
        let body = r#"<html><head><meta name="description" content="" /></head><body>
          <ul>
            <li><span class="pos">v.</span><span class="def">放弃；抛弃</span></li>
          </ul>
        </body></html>"#;
        let entry = parse_bing(body, "abandon").unwrap();
        assert_eq!(entry.definitions.len(), 2);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert_eq!(entry.definitions[1].meaning, "抛弃");
        assert_eq!(entry.phonetic, None);
    }

    #[test]
    fn page_without_entries_is_not_found() {
        let body = r#"<html><head><meta name="description" content="nothing" /></head></html>"#;
        assert_eq!(
            parse_bing(body, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
