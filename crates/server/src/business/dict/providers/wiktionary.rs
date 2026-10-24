use async_trait::async_trait;
use serde::Deserialize;

use crate::business::dict::entry::DictEntry;
use crate::business::dict::http::{DictHttp, NO_HEADERS};
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// 维基词典（`en.wiktionary.org/api/rest_v1/page/definition`，REST JSON）。
pub struct WiktionaryProvider;

#[async_trait]
impl DictProvider for WiktionaryProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Wiktionary
    }

    fn label(&self) -> &'static str {
        DictProviderId::Wiktionary.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://en.wiktionary.org/api/rest_v1/page/definition/{}",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &NO_HEADERS).await?;
        parse_wiktionary(&body, word)
    }
}

#[derive(Deserialize)]
struct WiktionaryResp {
    en: Option<Vec<WiktionarySection>>,
}

#[derive(Deserialize)]
struct WiktionarySection {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    definitions: Option<Vec<WiktionaryDefinition>>,
}

#[derive(Deserialize)]
struct WiktionaryDefinition {
    definition: Option<String>,
}

/// 解析维基词典定义接口：英文词性映射到白名单，释义 HTML 去标签/实体。
pub fn parse_wiktionary(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let resp: WiktionaryResp = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("维基词典响应解析失败: {e}")))?;
    let sections = resp.en.ok_or(DictProviderError::NotFound)?;
    let mut definitions = Vec::new();
    for section in sections {
        let pos = section
            .part_of_speech
            .as_deref()
            .and_then(pos::map_english_pos)
            .unwrap_or("");
        for definition in section.definitions.unwrap_or_default() {
            if let Some(raw) = definition.definition.as_deref() {
                pos::add_definition(&mut definitions, pos, &text::clean_html_text(raw));
            }
        }
    }
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::Wiktionary,
        label: DictProviderId::Wiktionary.label().to_string(),
        word: word.to_string(),
        phonetic: None,
        definitions,
        examples: Vec::new(),
        audio_url: None,
        source_url: Some(format!(
            "https://en.wiktionary.org/wiki/{}",
            text::url_encode(word)
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_wiktionary;

    const FIXTURE: &str = r#"{
      "en": [
        {
          "partOfSpeech": "Verb",
          "definitions": [
            {
              "definition": "<span class=\"usage-label-sense\"></span>To <a href=\"/wiki/give_up\" title=\"give up\">give up</a> with the intent of never again claiming a right.<style>.mw-parser-output .defdate{font-size:smaller}</style>"
            }
          ]
        },
        {
          "partOfSpeech": "Noun",
          "definitions": [{ "definition": "A yielding to natural impulses &amp; desires." }]
        }
      ]
    }"#;

    #[test]
    fn strips_html_and_decodes_entities() {
        let entry = parse_wiktionary(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Wiktionary);
        assert_eq!(entry.label, "维基词典");
        assert_eq!(entry.definitions.len(), 2);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert!(entry.definitions[0].meaning.starts_with("To give up"));
        assert!(!entry.definitions[0].meaning.contains('<'));
        assert!(
            !entry.definitions[0].meaning.contains("mw-parser-output"),
            "内联 <style> 内容不应混入释义"
        );
        assert_eq!(entry.definitions[1].pos, "n.");
        assert!(entry.definitions[1].meaning.contains("impulses & desires"));
        assert_eq!(
            entry.source_url.as_deref(),
            Some("https://en.wiktionary.org/wiki/abandon")
        );
    }

    #[test]
    fn missing_english_section_is_not_found() {
        assert_eq!(
            parse_wiktionary(r#"{"en":[]}"#, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
        assert_eq!(
            parse_wiktionary(r#"{}"#, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
