use async_trait::async_trait;
use serde::Deserialize;

use crate::business::dict::entry::DictEntry;
use crate::business::dict::example::DictExample;
use crate::business::dict::http::{DictHttp, NO_HEADERS};
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// Free Dictionary（`api.dictionaryapi.dev`，开放 JSON 接口）。
pub struct FreeDictProvider;

#[async_trait]
impl DictProvider for FreeDictProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::FreeDict
    }

    fn label(&self) -> &'static str {
        DictProviderId::FreeDict.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://api.dictionaryapi.dev/api/v2/entries/en/{}",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &NO_HEADERS).await?;
        parse_freedict(&body, word)
    }
}

#[derive(Deserialize)]
struct FreeEntry {
    phonetic: Option<String>,
    phonetics: Option<Vec<FreePhonetic>>,
    meanings: Option<Vec<FreeMeaning>>,
}

#[derive(Deserialize)]
struct FreePhonetic {
    text: Option<String>,
    audio: Option<String>,
}

#[derive(Deserialize)]
struct FreeMeaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    definitions: Option<Vec<FreeDefinition>>,
}

#[derive(Deserialize)]
struct FreeDefinition {
    definition: Option<String>,
    example: Option<String>,
}

/// 解析 Free Dictionary 响应：英文词性映射到白名单，例句取前 3 条，音频取首个非空。
pub fn parse_freedict(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let entries: Vec<FreeEntry> = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("Free Dictionary 响应解析失败: {e}")))?;
    let Some(entry) = entries.into_iter().next() else {
        return Err(DictProviderError::NotFound);
    };
    let phonetics = entry.phonetics.unwrap_or_default();
    let phonetic = entry
        .phonetic
        .as_deref()
        .and_then(text::normalize_phonetic)
        .or_else(|| {
            phonetics
                .iter()
                .find_map(|item| item.text.as_deref().and_then(text::normalize_phonetic))
        });
    let audio_url = phonetics.iter().find_map(|item| {
        item.audio
            .as_deref()
            .map(str::trim)
            .filter(|audio| !audio.is_empty())
            .map(str::to_string)
    });

    let mut definitions = Vec::new();
    let mut examples: Vec<DictExample> = Vec::new();
    for meaning in entry.meanings.unwrap_or_default() {
        let pos = meaning
            .part_of_speech
            .as_deref()
            .and_then(pos::map_english_pos)
            .unwrap_or("");
        for definition in meaning.definitions.unwrap_or_default() {
            if let Some(raw) = definition.definition.as_deref() {
                pos::add_definition(&mut definitions, pos, &text::clean_html_text(raw));
            }
            if examples.len() < 3 {
                if let Some(raw) = definition.example.as_deref() {
                    let example = text::clean_html_text(raw);
                    if !example.is_empty() {
                        examples.push(DictExample {
                            text: example,
                            translation: None,
                        });
                    }
                }
            }
        }
    }
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::FreeDict,
        label: DictProviderId::FreeDict.label().to_string(),
        word: word.to_string(),
        phonetic,
        definitions,
        examples,
        audio_url,
        source_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_freedict;

    const FIXTURE: &str = r#"[
      {
        "word": "abandon",
        "phonetic": "/əˈbændən/",
        "phonetics": [
          { "text": "/əˈbændən/", "audio": "" },
          { "text": "/əˈbændən/", "audio": "https://api.dictionaryapi.dev/media/abandon-us.mp3" }
        ],
        "meanings": [
          {
            "partOfSpeech": "verb",
            "definitions": [
              {
                "definition": "To give up with the intent of never again claiming a right or interest in.",
                "example": "He abandoned his car."
              },
              { "definition": "To cast aside." }
            ]
          },
          {
            "partOfSpeech": "noun",
            "definitions": [{ "definition": "A yielding to natural impulses." }]
          }
        ]
      }
    ]"#;

    #[test]
    fn parses_meanings_and_audio() {
        let entry = parse_freedict(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::FreeDict);
        assert_eq!(entry.label, "Free Dictionary");
        assert_eq!(entry.phonetic.as_deref(), Some("/əˈbændən/"));
        assert_eq!(
            entry.audio_url.as_deref(),
            Some("https://api.dictionaryapi.dev/media/abandon-us.mp3")
        );
        assert_eq!(entry.definitions.len(), 3);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert!(entry.definitions[0].meaning.starts_with("To give up"));
        assert_eq!(entry.definitions[2].pos, "n.");
        assert_eq!(entry.examples.len(), 1);
        assert_eq!(entry.examples[0].text, "He abandoned his car.");
    }

    #[test]
    fn empty_array_is_not_found() {
        assert_eq!(
            parse_freedict("[]", "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
