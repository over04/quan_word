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

/// 有道词典（`dict.youdao.com/jsonapi`，JSON 接口）。
pub struct YoudaoProvider;

#[async_trait]
impl DictProvider for YoudaoProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Youdao
    }

    fn label(&self) -> &'static str {
        DictProviderId::Youdao.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://dict.youdao.com/jsonapi?q={}&le=en",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &NO_HEADERS).await?;
        parse_youdao(&body, word)
    }
}

#[derive(Deserialize)]
struct YoudaoResp {
    ec: Option<YoudaoEc>,
    #[serde(rename = "blng_sents_part")]
    sentences: Option<YoudaoSentences>,
}

#[derive(Deserialize)]
struct YoudaoEc {
    word: Option<Vec<YoudaoWord>>,
}

#[derive(Deserialize)]
struct YoudaoWord {
    usphone: Option<String>,
    ukphone: Option<String>,
    trs: Option<Vec<YoudaoTrs>>,
}

#[derive(Deserialize)]
struct YoudaoTrs {
    tr: Option<Vec<YoudaoTr>>,
}

#[derive(Deserialize)]
struct YoudaoTr {
    l: Option<YoudaoTrLine>,
}

#[derive(Deserialize)]
struct YoudaoTrLine {
    /// 释义文本：单条是字符串，多条是字符串数组
    i: Option<YoudaoI18n>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum YoudaoI18n {
    One(String),
    Many(Vec<String>),
}

#[derive(Deserialize)]
struct YoudaoSentences {
    #[serde(rename = "sentence-pair")]
    pairs: Option<Vec<YoudaoSentencePair>>,
}

#[derive(Deserialize)]
struct YoudaoSentencePair {
    sentence: Option<String>,
    #[serde(rename = "sentence-translation")]
    translation: Option<String>,
}

/// 解析有道 jsonapi 响应：音标取美音优先，释义按 POS 前缀切条，例句取前 3 条。
pub fn parse_youdao(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let resp: YoudaoResp = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("有道响应解析失败: {e}")))?;
    let mut words = resp.ec.and_then(|ec| ec.word).unwrap_or_default();
    if words.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    let Some(first) = words.drain(..).next() else {
        return Err(DictProviderError::NotFound);
    };

    let phonetic = first
        .usphone
        .as_deref()
        .or(first.ukphone.as_deref())
        .and_then(text::normalize_phonetic);

    let mut block = String::new();
    for trs in first.trs.unwrap_or_default() {
        for tr in trs.tr.unwrap_or_default() {
            let Some(line) = tr.l else { continue };
            let Some(i18n) = line.i else { continue };
            let lines = match i18n {
                YoudaoI18n::One(line) => vec![line],
                YoudaoI18n::Many(lines) => lines,
            };
            for line in lines {
                block.push_str(&text::clean_html_text(&line));
                block.push('\n');
            }
        }
    }
    let definitions = pos::pos_definitions(&block, &[';', '；']);
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }

    let examples = resp
        .sentences
        .and_then(|part| part.pairs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|pair| {
            let sentence = pair.sentence?;
            let sentence_text = text::clean_html_text(&sentence);
            if sentence_text.is_empty() {
                return None;
            }
            let translation = pair
                .translation
                .map(|raw| text::clean_html_text(&raw))
                .filter(|translated| !translated.is_empty());
            Some(DictExample {
                text: sentence_text,
                translation,
            })
        })
        .take(3)
        .collect();

    Ok(DictEntry {
        provider: DictProviderId::Youdao,
        label: DictProviderId::Youdao.label().to_string(),
        word: word.to_string(),
        phonetic,
        definitions,
        examples,
        audio_url: None,
        source_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_youdao;

    const FIXTURE: &str = r#"{
      "ec": {
        "word": [
          {
            "usphone": "əˈbændən",
            "ukphone": "əˈbændən",
            "trs": [
              { "tr": [ { "l": { "i": ["vt. 放弃，抛弃；离弃", "n. 放任，放纵"] } } ] }
            ]
          }
        ]
      },
      "blng_sents_part": {
        "sentence-pair": [
          { "sentence": "He <b>abandoned</b> his car.", "sentence-translation": "他丢弃了汽车。" },
          { "sentence": "She abandoned hope.", "sentence-translation": "她放弃了希望。" },
          { "sentence": "They abandoned the plan.", "sentence-translation": "他们放弃了计划。" },
          { "sentence": "Too many sentences.", "sentence-translation": "超出三条。" }
        ]
      }
    }"#;

    #[test]
    fn parses_phonetic_definitions_and_examples() {
        let entry = parse_youdao(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Youdao);
        assert_eq!(entry.label, "有道词典");
        assert_eq!(entry.phonetic.as_deref(), Some("/əˈbændən/"));
        assert_eq!(entry.definitions.len(), 3);
        assert_eq!(entry.definitions[0].pos, "vt.");
        assert_eq!(entry.definitions[0].meaning, "放弃，抛弃");
        assert_eq!(entry.definitions[2].pos, "n.");
        assert_eq!(entry.examples.len(), 3);
        assert_eq!(entry.examples[0].text, "He abandoned his car.");
        assert_eq!(
            entry.examples[0].translation.as_deref(),
            Some("他丢弃了汽车。")
        );
    }

    #[test]
    fn empty_ec_is_not_found() {
        let err = parse_youdao(r#"{"ec":{}}"#, "abandon").unwrap_err();
        assert_eq!(err, super::DictProviderError::NotFound);
        let err = parse_youdao(r#"{"ec":{"word":[]}}"#, "abandon").unwrap_err();
        assert_eq!(err, super::DictProviderError::NotFound);
    }

    #[test]
    fn broken_json_is_parse_error() {
        let err = parse_youdao("not json", "abandon").unwrap_err();
        assert!(matches!(err, super::DictProviderError::Parse(_)));
    }
}
