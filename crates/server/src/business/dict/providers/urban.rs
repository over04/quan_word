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

/// Urban Dictionary（`api.urbandictionary.com/v0/define`，俚语 JSON 接口）。
pub struct UrbanProvider;

#[async_trait]
impl DictProvider for UrbanProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Urban
    }

    fn label(&self) -> &'static str {
        DictProviderId::Urban.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://api.urbandictionary.com/v0/define?term={}",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &NO_HEADERS).await?;
        parse_urban(&body, word)
    }
}

#[derive(Deserialize)]
struct UrbanResp {
    list: Option<Vec<UrbanItem>>,
}

#[derive(Deserialize)]
struct UrbanItem {
    definition: Option<String>,
    example: Option<String>,
}

/// 解析 Urban Dictionary 响应：取前 3 条定义（无词性），去掉 [ ] 链接标记。
pub fn parse_urban(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let resp: UrbanResp = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("Urban Dictionary 响应解析失败: {e}")))?;
    let items = resp.list.unwrap_or_default();
    if items.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    let mut definitions = Vec::new();
    let mut examples: Vec<DictExample> = Vec::new();
    for item in items.iter().take(3) {
        if let Some(raw) = item.definition.as_deref() {
            pos::add_definition(&mut definitions, "", &clean_urban_text(raw));
        }
        if examples.len() < 2 {
            if let Some(raw) = item.example.as_deref() {
                let example = clean_urban_text(raw);
                if !example.is_empty() {
                    examples.push(DictExample {
                        text: example,
                        translation: None,
                    });
                }
            }
        }
    }
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::Urban,
        label: DictProviderId::Urban.label().to_string(),
        word: word.to_string(),
        phonetic: None,
        definitions,
        examples,
        audio_url: None,
        source_url: Some(format!(
            "https://www.urbandictionary.com/define.php?term={}",
            text::url_encode(word)
        )),
    })
}

/// 清洗 Urban 文本：去掉 `[词条]` 的方括号标记、HTML 标签与换行。
fn clean_urban_text(raw: &str) -> String {
    text::clean_html_text(&raw.replace(['[', ']'], ""))
}

#[cfg(test)]
mod tests {
    use super::parse_urban;

    const FIXTURE: &str = r#"{
      "list": [
        {
          "definition": "To [leave] someone or something behind.\r\nUsually permanent.",
          "example": "I [abandoned] my [car]."
        },
        { "definition": "Total freedom.", "example": "Dance like nobody's watching." },
        { "definition": "Third definition.", "example": "Third example." },
        { "definition": "Fourth definition.", "example": "Fourth example." }
      ]
    }"#;

    #[test]
    fn parses_first_three_definitions_without_brackets() {
        let entry = parse_urban(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Urban);
        assert_eq!(entry.label, "Urban Dictionary");
        assert_eq!(entry.definitions.len(), 3);
        assert_eq!(entry.definitions[0].pos, "");
        assert_eq!(
            entry.definitions[0].meaning,
            "To leave someone or something behind. Usually permanent."
        );
        assert_eq!(entry.definitions[2].meaning, "Third definition.");
        assert_eq!(entry.examples.len(), 2);
        assert_eq!(entry.examples[0].text, "I abandoned my car.");
        assert_eq!(
            entry.source_url.as_deref(),
            Some("https://www.urbandictionary.com/define.php?term=abandon")
        );
    }

    #[test]
    fn empty_list_is_not_found() {
        assert_eq!(
            parse_urban(r#"{"list":[]}"#, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
