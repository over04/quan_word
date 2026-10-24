use std::sync::LazyLock;

use async_trait::async_trait;
use scraper::{Html, Selector};

use crate::business::dict::entry::DictEntry;
use crate::business::dict::example::DictExample;
use crate::business::dict::http::DictHttp;
use crate::business::dict::http_header::HttpHeader;
use crate::business::dict::pos;
use crate::business::dict::provider::DictProvider;
use crate::business::dict::provider_error::DictProviderError;
use crate::business::dict::provider_id::DictProviderId;
use crate::business::dict::text;

/// 欧路词典（`dict.eudic.net/minidicts/en/{word}`，HTML 迷你词条页）。
pub struct EudicProvider;

/// 欧路对缺少站内 Referer 的请求返回空页。
const HEADERS: [HttpHeader; 1] = [HttpHeader::new("Referer", "https://dict.eudic.net/")];

#[async_trait]
impl DictProvider for EudicProvider {
    fn id(&self) -> DictProviderId {
        DictProviderId::Eudic
    }

    fn label(&self) -> &'static str {
        DictProviderId::Eudic.label()
    }

    async fn lookup(&self, http: &DictHttp, word: &str) -> Result<DictEntry, DictProviderError> {
        let url = format!(
            "https://dict.eudic.net/minidicts/en/{}",
            text::url_encode(word)
        );
        let body = http.get_text(&url, &HEADERS).await?;
        parse_eudic(&body, word)
    }
}

/// 编译一次的选择器（进程内复用，不在热路径重复解析）。
///
/// 页面有多本词典：`#FCchild` 是英汉-汉英词典，`#FFchild` 是英英词典，
/// `#DictLijuchild` 是例句区——释义只取第一本，避免混入英文长句。
static PHONETIC_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("span.phonitic-line"));
static US_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("us"));
static UK_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("uk"));
static DEFINITION_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| selector("#FCchild .exp > ol > li"));
static EXAMPLE_ITEM_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| selector("#DictLijuchild .lj_item"));
static EXAMPLE_LINE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("p.line"));
static EXAMPLE_TRANSLATION_SELECTOR: LazyLock<Selector> = LazyLock::new(|| selector("p.exp"));

fn selector(raw: &str) -> Selector {
    Selector::parse(raw).expect("欧路词典 CSS 选择器应合法")
}

/// 解析欧路迷你词条页：音标优先美音，释义取英汉-汉英词典，例句取前 2 条。
pub fn parse_eudic(body: &str, word: &str) -> Result<DictEntry, DictProviderError> {
    let document = Html::parse_document(body);

    let phonetic = document
        .select(&PHONETIC_SELECTOR)
        .next()
        .and_then(|block| {
            let us = block.select(&US_SELECTOR).next().map(text::element_text);
            let uk = block.select(&UK_SELECTOR).next().map(text::element_text);
            us.as_deref()
                .or(uk.as_deref())
                .and_then(text::normalize_phonetic)
        });

    // 各 li 是一行释义，词性只出现在部分行首：用「；」串成一段再统一解析，
    // 没有词性前缀的行自然继承前一个词性。
    let mut block = String::new();
    for element in document.select(&DEFINITION_SELECTOR) {
        let line = text::element_text(element);
        if line.is_empty() {
            continue;
        }
        if !block.is_empty() {
            block.push('；');
        }
        block.push_str(&line);
    }
    let definitions = pos::pos_definitions(&block, &['；', ';']);
    if definitions.is_empty() {
        return Err(DictProviderError::Parse("欧路词典页面未解析到释义".into()));
    }

    let examples = document
        .select(&EXAMPLE_ITEM_SELECTOR)
        .filter_map(|item| {
            let sentence = text::element_text(item.select(&EXAMPLE_LINE_SELECTOR).next()?);
            if sentence.is_empty() {
                return None;
            }
            let translation = item
                .select(&EXAMPLE_TRANSLATION_SELECTOR)
                .next()
                .map(text::element_text)
                .filter(|translated| !translated.is_empty());
            Some(DictExample {
                text: sentence,
                translation,
            })
        })
        .take(2)
        .collect();

    Ok(DictEntry {
        provider: DictProviderId::Eudic,
        label: DictProviderId::Eudic.label().to_string(),
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
    use super::parse_eudic;

    /// 真实页面片段（abandon）：英英词典区 `#FFchild` 与例句区不参与释义。
    const FIXTURE: &str = r#"<html><body>
      <span class="phonitic-line minDictSearchPhon"><uk>/&#601;&#712;b&aelig;nd&#601;n/</uk><us>/əˈbændən/</us></span>
      <div id="FC" class="explain_wrap">
        <div id="FCchild" class="expDiv">
          <div class="exp"><ol>
            <li>v. （不顾责任、义务等）离弃，遗弃，抛弃</li>
            <li>（不得已而）舍弃，丢弃，离开</li>
            <li>停止（支持或帮助）；放弃（信念）</li>
            <li>n. 【正式】放任；放纵</li>
          </ol></div>
          <div id="trans"><span class="txtDisabled">时 态:  </span><span class="trans">abandoned, abandoning, abandons</span></div>
        </div>
      </div>
      <div id="FF" class="explain_wrap">
        <div id="FFchild" class="expDiv">
          <div class="exp"><ol><li>To give up with the intent of never again claiming a right.</li></ol></div>
        </div>
      </div>
      <div id="DictLijuchild" class="expDiv">
        <div id="lj_div" class="sentence">
          <div class="lj_item" type="dict">
            <div class="content">
              <p class="line">After careful deliberation, it was agreed to <span class="key">abandon</span> the project.</p>
              <p class="exp">经过慎重考虑，大家同意<span class="key">放弃</span>这个项目。</p>
            </div>
          </div>
          <div class="lj_item" type="dict">
            <div class="content">
              <p class="line">One morning we came upon an <span class="key">abandoned</span> undernourished puppy.</p>
              <p class="exp">一天早上我们发现了一只营养不良、被人<span class="key">抛弃</span>的小狗。</p>
            </div>
          </div>
        </div>
      </div>
    </body></html>"#;

    #[test]
    fn parses_phonetic_definitions_and_examples() {
        let entry = parse_eudic(FIXTURE, "abandon").unwrap();
        assert_eq!(entry.provider, super::DictProviderId::Eudic);
        assert_eq!(entry.label, "欧路词典");
        assert_eq!(entry.phonetic.as_deref(), Some("/əˈbændən/"));
        assert_eq!(entry.definitions.len(), 6);
        assert_eq!(entry.definitions[0].pos, "v.");
        assert_eq!(
            entry.definitions[0].meaning,
            "（不顾责任、义务等）离弃，遗弃，抛弃"
        );
        assert_eq!(entry.definitions[1].pos, "v.", "无词性的行继承上一行");
        assert_eq!(entry.definitions[1].meaning, "（不得已而）舍弃，丢弃，离开");
        assert_eq!(entry.definitions[2].meaning, "停止（支持或帮助）");
        assert_eq!(entry.definitions[3].meaning, "放弃（信念）");
        assert_eq!(entry.definitions[4].pos, "n.");
        assert_eq!(entry.definitions[4].meaning, "【正式】放任");
        assert_eq!(entry.definitions[5].meaning, "放纵");
        for definition in &entry.definitions {
            assert!(
                !definition.meaning.contains('\n'),
                "释义不应包含换行: {:?}",
                definition.meaning
            );
            assert!(
                !definition.meaning.contains("To give up"),
                "不应混入英英词典释义"
            );
        }
        assert_eq!(entry.examples.len(), 2);
        assert_eq!(
            entry.examples[0].text,
            "After careful deliberation, it was agreed to abandon the project."
        );
        assert_eq!(
            entry.examples[0].translation.as_deref(),
            Some("经过慎重考虑，大家同意放弃这个项目。")
        );
    }

    #[test]
    fn empty_definition_list_is_parse_error() {
        let body = r#"<span class="phonitic-line">/x/</span><div id="FCchild"><div class="exp"><ol></ol></div></div>"#;
        let err = parse_eudic(body, "abandon").unwrap_err();
        assert!(matches!(err, super::DictProviderError::Parse(_)));
    }
}
