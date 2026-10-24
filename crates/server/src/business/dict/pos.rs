//! 词性解析：把各词典源的词性标记统一到单词本的白名单书写形式。
//!
//! 复用 `words::pos::WordPos`（21 种书写形式 → 18 个语义变体）；
//! 中文串里的 POS 边界识别、英文 POS 映射、释义去重/截断/上限都集中在这里。
//! 边界识别是纯字符扫描（无正则），词性标记表按长度降序匹配。

use std::str::FromStr;

use entity::definition::Definition;

use crate::business::wordbooks::words::pos::WordPos;

use super::pos_form::PosForm;
use super::pos_mark::PosMark;
use super::pos_segment::PosSegment;
use super::pos_token::PosToken;
use super::text::{cap_chars, collapse_ws, MAX_MEANING_CHARS};

/// 单个源最多返回的释义条数。
pub const MAX_DEFINITIONS: usize = 20;

/// 词性枚举 → 白名单书写形式（同义书写 c/C、u/U、cu/CU 统一取首个）。
const POS_FORMS: [PosForm; 18] = [
    PosForm::new(WordPos::N, "n."),
    PosForm::new(WordPos::Countable, "c"),
    PosForm::new(WordPos::Uncountable, "u"),
    PosForm::new(WordPos::Both, "cu"),
    PosForm::new(WordPos::V, "v."),
    PosForm::new(WordPos::Vt, "vt."),
    PosForm::new(WordPos::Vi, "vi."),
    PosForm::new(WordPos::Adj, "adj."),
    PosForm::new(WordPos::Adv, "adv."),
    PosForm::new(WordPos::Prep, "prep."),
    PosForm::new(WordPos::Conj, "conj."),
    PosForm::new(WordPos::Pron, "pron."),
    PosForm::new(WordPos::Num, "num."),
    PosForm::new(WordPos::Art, "art."),
    PosForm::new(WordPos::Interj, "interj."),
    PosForm::new(WordPos::Aux, "aux."),
    PosForm::new(WordPos::Abbr, "abbr."),
    PosForm::new(WordPos::Phr, "phr."),
];

/// 中文释义串里可识别的 POS 缩写 → 白名单书写形式。
///
/// 顺序按长度降序：保证 `vt.`/`vi.` 不会被 `v.` 抢先匹配。
const POS_TOKENS: [PosToken; 15] = [
    PosToken::new("interj", "interj."),
    PosToken::new("abbr", "abbr."),
    PosToken::new("pron", "pron."),
    PosToken::new("conj", "conj."),
    PosToken::new("prep", "prep."),
    PosToken::new("adj", "adj."),
    PosToken::new("adv", "adv."),
    PosToken::new("num", "num."),
    PosToken::new("art", "art."),
    PosToken::new("aux", "aux."),
    PosToken::new("phr", "phr."),
    PosToken::new("vt", "vt."),
    PosToken::new("vi", "vi."),
    PosToken::new("v", "v."),
    PosToken::new("n", "n."),
];

/// 把词性标记规范化为白名单书写形式；无法识别返回 None。
///
/// 支持缩写（`v.` / `N` / `vt`）、英文全称（`verb` / `Auxiliary verb`）。
pub fn canonical_pos(token: &str) -> Option<&'static str> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if let Ok(pos) = WordPos::from_str(&lower) {
        return Some(form_of(pos));
    }
    if let Some(form) = map_english_pos(&lower) {
        return Some(form);
    }
    WordPos::from_str(&format!("{lower}.")).ok().map(form_of)
}

/// 英文词性名称 → 白名单书写形式（freedict / wiktionary 使用）。
pub fn map_english_pos(raw: &str) -> Option<&'static str> {
    let key = raw.trim().to_lowercase();
    let key = key.trim_end_matches('.');
    Some(match key {
        "noun" | "proper noun" | "countable noun" | "uncountable noun" => "n.",
        "verb" | "transitive verb" | "intransitive verb" => "v.",
        "adjective" => "adj.",
        "adverb" => "adv.",
        "pronoun" => "pron.",
        "preposition" => "prep.",
        "conjunction" => "conj.",
        "interjection" => "interj.",
        "numeral" | "number" => "num.",
        "article" | "determiner" => "art.",
        "auxiliary verb" | "auxiliary" => "aux.",
        "phrase" | "idiom" | "proverb" => "phr.",
        "abbreviation" | "initialism" | "acronym" => "abbr.",
        _ => return None,
    })
}

/// 按 POS 边界切分中文释义串：返回每个词性片段（词性 + 原文）。
pub fn split_pos_segments(text: &str) -> Vec<PosSegment> {
    let marks = find_pos_marks(text);
    if marks.is_empty() {
        let all = text.trim();
        return if all.is_empty() {
            Vec::new()
        } else {
            vec![PosSegment {
                pos: String::new(),
                text: all.to_string(),
            }]
        };
    }
    let mut segments = Vec::with_capacity(marks.len() + 1);
    let lead = text[..marks[0].start].trim();
    if !lead.is_empty() {
        segments.push(PosSegment {
            pos: String::new(),
            text: lead.to_string(),
        });
    }
    for (index, mark) in marks.iter().enumerate() {
        let next_start = marks.get(index + 1).map_or(text.len(), |next| next.start);
        segments.push(PosSegment {
            pos: mark.form.to_string(),
            text: text[mark.end..next_start].trim().to_string(),
        });
    }
    segments
}

/// 解析「POS 前缀 + 分隔符」的释义串（百度/金山/欧路等中文上游）。
///
/// 先按 POS 切段，段内再按 `separators` 拆条；每条截断到 300 字符、去重、最多 20 条。
pub fn pos_definitions(text: &str, separators: &[char]) -> Vec<Definition> {
    let mut definitions = Vec::new();
    for segment in split_pos_segments(text) {
        for piece in segment.text.split(|c: char| separators.contains(&c)) {
            add_definition(&mut definitions, &segment.pos, piece);
        }
    }
    definitions
}

/// 追加一条释义（去边界分隔符 + 截断 + 去重 + 上限）；返回是否实际追加。
///
/// 边界上的 `;`/`；`/`,`/`，` 来自上游分段残留（如 POS 边界处的分号），统一剥掉。
pub fn add_definition(definitions: &mut Vec<Definition>, pos: &str, meaning: &str) -> bool {
    if definitions.len() >= MAX_DEFINITIONS {
        return false;
    }
    let meaning =
        meaning.trim_matches(|c: char| c.is_whitespace() || matches!(c, ';' | '；' | ',' | '，'));
    // 上游多行释义（欧路 li 内含换行）统一折叠为单行
    let meaning = cap_chars(&collapse_ws(meaning), MAX_MEANING_CHARS);
    if meaning.is_empty() {
        return false;
    }
    if definitions
        .iter()
        .any(|existing| existing.pos == pos && existing.meaning == meaning)
    {
        return false;
    }
    definitions.push(Definition {
        pos: pos.to_string(),
        meaning,
    });
    true
}

fn form_of(pos: WordPos) -> &'static str {
    POS_FORMS
        .iter()
        .find(|candidate| candidate.pos == pos)
        .map_or("", |candidate| candidate.form)
}

/// 扫描全部 POS 标记，按出现顺序返回。
fn find_pos_marks(text: &str) -> Vec<PosMark> {
    let mut marks = Vec::new();
    let mut prev: Option<char> = None;
    for (start, ch) in text.char_indices() {
        let at_boundary = prev.is_none_or(|prev| {
            prev.is_whitespace() || matches!(prev, ';' | '；' | ',' | '，' | '|')
        });
        if at_boundary {
            if let Some(mark) = match_pos_token(text, start) {
                marks.push(mark);
            }
        }
        prev = Some(ch);
    }
    marks
}

/// 在 `start` 处匹配一个 POS 缩写 + `.`（ASCII 大小写不敏感）。
fn match_pos_token(text: &str, start: usize) -> Option<PosMark> {
    let rest = &text[start..];
    for token in POS_TOKENS {
        let Some(head) = rest.get(..token.token.len()) else {
            continue;
        };
        if !head.eq_ignore_ascii_case(token.token) {
            continue;
        }
        if rest[token.token.len()..].starts_with('.') {
            return Some(PosMark {
                start,
                end: start + token.token.len() + 1,
                form: token.form,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{
        add_definition, canonical_pos, map_english_pos, pos_definitions, split_pos_segments,
        WordPos, POS_FORMS,
    };
    use crate::business::dict::pos_segment::PosSegment;

    #[test]
    fn canonical_forms_cover_whitelist() {
        assert_eq!(canonical_pos("v."), Some("v."));
        assert_eq!(canonical_pos("N"), Some("n."));
        assert_eq!(canonical_pos("vt"), Some("vt."));
        assert_eq!(canonical_pos(" adj. "), Some("adj."));
        assert_eq!(canonical_pos("C"), Some("c"));
        assert_eq!(canonical_pos("cu"), Some("cu"));
    }

    #[test]
    fn canonical_forms_round_trip_through_word_pos() {
        // 表内书写形式必须能被 WordPos 解析回同一变体；Display 输出同样可回环（含 C/U/CU 同义写法）
        for entry in POS_FORMS {
            assert_eq!(
                WordPos::from_str(entry.form).unwrap(),
                entry.pos,
                "词性表书写形式应可解析: {}",
                entry.form
            );
            assert_eq!(
                WordPos::from_str(&entry.pos.to_string()).unwrap(),
                entry.pos,
                "WordPos::Display 应可回环"
            );
        }
    }

    #[test]
    fn maps_english_names() {
        assert_eq!(map_english_pos("Verb"), Some("v."));
        assert_eq!(map_english_pos("auxiliary verb"), Some("aux."));
        assert_eq!(map_english_pos("Proper Noun"), Some("n."));
        assert_eq!(map_english_pos("interjection"), Some("interj."));
        assert_eq!(map_english_pos("particle"), None);
        assert_eq!(canonical_pos("particle"), None);
        assert_eq!(canonical_pos(""), None);
    }

    #[test]
    fn splits_chinese_pos_segments() {
        let segments = split_pos_segments("vt. 放弃，抛弃; 离弃 n. 放任，放纵");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].pos, "vt.");
        assert_eq!(segments[0].text, "放弃，抛弃; 离弃");
        assert_eq!(segments[1].pos, "n.");
        assert_eq!(segments[1].text, "放任，放纵");
    }

    #[test]
    fn keeps_leading_text_without_pos() {
        let segments = split_pos_segments("放弃；抛弃");
        assert_eq!(
            segments,
            vec![PosSegment {
                pos: String::new(),
                text: "放弃；抛弃".to_string()
            }]
        );
        assert!(split_pos_segments("   ").is_empty());
    }

    #[test]
    fn ignores_pos_inside_words() {
        // attention. 里的 n. 不是词性标记
        let segments = split_pos_segments("Pay attention.");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].pos, "");
        // 大写缩写同样可识别
        let upper = split_pos_segments("VT. 放弃");
        assert_eq!(upper[0].pos, "vt.");
        assert_eq!(upper[0].text, "放弃");
    }

    #[test]
    fn builds_definitions_with_pos_and_dedup() {
        let defs = pos_definitions("vt. 放弃，抛弃; 离弃，丢掷 n. 放任，放纵", &[';', '；']);
        assert_eq!(defs.len(), 3);
        assert_eq!(defs[0].pos, "vt.");
        assert_eq!(defs[0].meaning, "放弃，抛弃");
        assert_eq!(defs[1].meaning, "离弃，丢掷");
        assert_eq!(defs[2].pos, "n.");
        assert_eq!(defs[2].meaning, "放任，放纵");
    }

    #[test]
    fn add_definition_truncates_and_caps() {
        let mut defs = Vec::new();
        assert!(add_definition(&mut defs, "n.", "  放任  "));
        assert_eq!(defs[0].meaning, "放任");
        assert!(!add_definition(&mut defs, "n.", "放任"), "重复释义应跳过");
        assert!(!add_definition(&mut defs, "n.", "   "), "空释义应跳过");

        assert!(add_definition(&mut defs, "v.", "放弃\n中止\t停止"));
        assert_eq!(defs[1].meaning, "放弃 中止 停止", "多行释义应折叠为单行");

        let long = "a".repeat(400);
        assert!(add_definition(&mut defs, "v.", &long));
        assert_eq!(defs[2].meaning.chars().count(), 300);

        let mut many = Vec::new();
        for i in 0..30 {
            add_definition(&mut many, "n.", &format!("释义{i}"));
        }
        assert_eq!(many.len(), super::MAX_DEFINITIONS);
    }
}
