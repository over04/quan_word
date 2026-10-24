use async_trait::async_trait;
use serde::Deserialize;

use crate::business::dict::entry::DictEntry;
use crate::business::dict::http::{DictHttp, NO_HEADERS};
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// 金山词霸（`dict.iciba.com/dictionary/word/suggestion`，JSON 接口）。
pub struct IcibaProvider;

#[async_trait]
impl DictProvider for IcibaProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Iciba
    }

    fn label(&self) -> &'static str {
        DictProviderId::Iciba.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://dict.iciba.com/dictionary/word/suggestion?word={}&nums=1",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &NO_HEADERS).await?;
        parse_iciba(&body, word)
    }
}

#[derive(Deserialize)]
struct IcibaResp {
    message: Option<Vec<IcibaItem>>,
}

#[derive(Deserialize)]
struct IcibaItem {
    paraphrase: Option<String>,
}

/// 解析金山词霸建议接口：`paraphrase` 形如 `v.抛弃，舍弃,中止;n.放任`。
pub fn parse_iciba(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let resp: IcibaResp = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("金山词霸响应解析失败: {e}")))?;
    let paraphrase = resp
        .message
        .unwrap_or_default()
        .into_iter()
        .find_map(|item| item.paraphrase)
        .ok_or(DictProviderError::NotFound)?;
    let definitions = pos::pos_definitions(&paraphrase, &[',', '，']);
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::Iciba,
        label: DictProviderId::Iciba.label().to_string(),
        word: word.to_string(),
        phonetic: None,
        definitions,
        examples: Vec::new(),
        audio_url: None,
        source_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_iciba;

    const FIXTURE: &str = r#"{
      "message": [
        { "key": "abandon", "paraphrase": "v.抛弃，舍弃,中止，放弃,放纵;n.放任，放纵" }
      ]
    }"#;

    #[test]
    fn parses_comma_separated_meanings() {
        let entry = parse_iciba(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Iciba);
        assert_eq!(entry.label, "金山词霸");
        assert_eq!(entry.definitions.len(), 7);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert_eq!(entry.definitions[0].meaning, "抛弃");
        assert_eq!(entry.definitions[1].meaning, "舍弃");
        assert_eq!(entry.definitions[2].meaning, "中止");
        assert_eq!(entry.definitions[4].meaning, "放纵");
        assert_eq!(entry.definitions[5].pos, "n.");
        assert_eq!(entry.definitions[5].meaning, "放任");
        assert_eq!(entry.definitions[6].meaning, "放纵");
        assert!(entry.examples.is_empty());
    }

    #[test]
    fn empty_message_is_not_found() {
        assert_eq!(
            parse_iciba(r#"{"message":[]}"#, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
