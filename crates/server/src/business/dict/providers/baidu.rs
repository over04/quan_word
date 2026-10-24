use async_trait::async_trait;
use serde::Deserialize;

use crate::business::dict::entry::DictEntry;
use crate::business::dict::form_body::FormBody;
use crate::business::dict::form_field::FormField;
use crate::business::dict::http::DictHttp;
use crate::business::dict::http_header::HttpHeader;
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// 百度翻译（`fanyi.baidu.com/sug`，表单 POST）。
pub struct BaiduProvider;

/// 百度对跨站表单请求校验 Referer。
const HEADERS: [HttpHeader; 1] = [HttpHeader::new("Referer", "https://fanyi.baidu.com/")];

#[async_trait]
impl DictProvider for BaiduProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Baidu
    }

    fn label(&self) -> &'static str {
        DictProviderId::Baidu.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let fields = [FormField::new("kw", word)];
        let body = http
            .post_form_text(
                "https://fanyi.baidu.com/sug",
                FormBody::new(&fields),
                &HEADERS,
            )
            .await?;
        parse_baidu(&body, word)
    }
}

#[derive(Deserialize)]
struct BaiduResp {
    data: Option<Vec<BaiduItem>>,
}

#[derive(Deserialize)]
struct BaiduItem {
    k: Option<String>,
    v: Option<String>,
}

/// 解析百度 sug 响应：优先取与查询词大小写不敏感相同的条目，否则取第一条。
pub fn parse_baidu(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let resp: BaiduResp = serde_json::from_str(body)
        .map_err(|e| DictProviderError::Parse(format!("百度翻译响应解析失败: {e}")))?;
    let items = resp.data.unwrap_or_default();
    let chosen = items
        .iter()
        .find(|item| {
            item.k
                .as_deref()
                .is_some_and(|key| key.eq_ignore_ascii_case(word))
        })
        .or_else(|| items.first())
        .ok_or(DictProviderError::NotFound)?;
    let raw = chosen.v.as_deref().unwrap_or("");
    let definitions = pos::pos_definitions(raw, &[';', '；']);
    if definitions.is_empty() {
        return Err(DictProviderError::NotFound);
    }
    Ok(DictEntry {
        provider: DictProviderId::Baidu,
        label: DictProviderId::Baidu.label().to_string(),
        word: chosen.k.clone().unwrap_or_else(|| word.to_string()),
        phonetic: None,
        definitions,
        examples: Vec::new(),
        audio_url: None,
        source_url: Some(format!(
            "https://fanyi.baidu.com/#en/zh/{}",
            text::url_encode(word)
        )),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_baidu;

    const FIXTURE: &str = r#"{
      "errno": 0,
      "data": [
        { "k": "abandon", "v": "vt. 放弃，抛弃; 离弃，丢掷; 中止，放弃; n. 放任，放纵" },
        { "k": "abandoned", "v": "adj. 被抛弃的" }
      ]
    }"#;

    #[test]
    fn parses_pos_segments_and_meanings() {
        let entry = parse_baidu(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Baidu);
        assert_eq!(entry.word, "abandon");
        assert_eq!(entry.phonetic, None);
        assert_eq!(entry.definitions.len(), 4);
        assert_eq!(entry.definitions[0].pos, "vt.");
        assert_eq!(entry.definitions[0].meaning, "放弃，抛弃");
        assert_eq!(entry.definitions[1].meaning, "离弃，丢掷");
        assert_eq!(entry.definitions[3].pos, "n.");
        assert_eq!(entry.definitions[3].meaning, "放任，放纵");
        assert_eq!(
            entry.source_url.as_deref(),
            Some("https://fanyi.baidu.com/#en/zh/abandon")
        );
    }

    #[test]
    fn falls_back_to_first_item() {
        let fixture = r#"{"data":[{"k":"other","v":"n. 其它"}]}"#;
        let entry = parse_baidu(fixture, "abandon").unwrap();
        assert_eq!(entry.word, "other");
        assert_eq!(entry.definitions[0].meaning, "其它");
    }

    #[test]
    fn empty_data_is_not_found() {
        assert_eq!(
            parse_baidu(r#"{"errno":0,"data":[]}"#, "abandon").unwrap_err(),
            super::DictProviderError::NotFound
        );
    }
}
