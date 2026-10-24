//! 词典文本清洗：HTML 片段取文本（html5ever）、空白折叠、按字符截断、音标与 URL 规范化。

use scraper::node::Node;
use scraper::{ElementRef, Html};

/// 单条释义的最大字符数（超出按字符截断）。
pub const MAX_MEANING_CHARS: usize = 300;

/// 折叠连续空白并去除首尾空白（`split_whitespace` 已覆盖 Unicode 空白，如全角空格）。
pub fn collapse_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 元素可见文本（递归收集文本节点，自动去标签、解实体）并折叠空白。
pub fn element_text(element: ElementRef<'_>) -> String {
    collapse_ws(&element.text().collect::<String>())
}

/// HTML 片段取纯文本：html5ever 解析后收集文本节点（自动去标签、解实体）。
///
/// 用于 JSON 里夹带 HTML 的字段（有道 `<b>`、Wiktionary definition 等）；
/// `<style>`/`<script>` 的内容不是释义，先摘除再取文本。
pub fn clean_html_text(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut fragment = Html::parse_fragment(input);
    strip_noise(&mut fragment);
    element_text(fragment.root_element())
}

/// 摘除 `<style>` / `<script>` 节点（Wiktionary 会在 definition 里内联 CSS）。
fn strip_noise(fragment: &mut Html) {
    let noisy: Vec<_> = fragment
        .tree
        .nodes()
        .filter(|node| match node.value() {
            Node::Element(element) => matches!(element.name(), "style" | "script"),
            _ => false,
        })
        .map(|node| node.id())
        .collect();
    for id in noisy {
        if let Some(mut node) = fragment.tree.get_mut(id) {
            node.detach();
        }
    }
}

/// 按字符数截断（CJK 与组合字符都按一个字符计）。
pub fn cap_chars(input: &str, max: usize) -> String {
    if input.chars().count() <= max {
        return input.to_string();
    }
    input.chars().take(max).collect()
}

/// 规范化音标：去掉已有方括号/斜杠后统一包成 `/.../`；空串返回 None。
pub fn normalize_phonetic(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '/'));
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("/{trimmed}/"))
    }
}

/// 取 `prefix` 与紧随其后的 `suffix` 之间的片段（纯字符串扫描，无正则）。
pub fn between<'a>(haystack: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let start = haystack.find(prefix)? + prefix.len();
    let rest = &haystack[start..];
    let end = rest.find(suffix)?;
    Some(&rest[..end])
}

/// URL 编码（查询参数与路径片段共用）。
pub fn url_encode(raw: &str) -> String {
    percent_encoding::utf8_percent_encode(raw, percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::{between, cap_chars, clean_html_text, collapse_ws, normalize_phonetic, url_encode};

    #[test]
    fn extracts_text_from_html_fragment() {
        assert_eq!(
            clean_html_text("He <b>abandoned</b> his car."),
            "He abandoned his car."
        );
        assert_eq!(
            clean_html_text("To <a href=\"/wiki/give_up\" title=\"give up\">give up</a> now."),
            "To give up now."
        );
        assert_eq!(clean_html_text("<img src=\"x.png\" />放弃"), "放弃");
        // 内联样式/脚本内容不是释义（Wiktionary definition 会带 <style>）
        assert_eq!(clean_html_text("<style>.x{}</style>释义"), "释义");
        assert_eq!(clean_html_text("放弃<script>var a = 1;</script>"), "放弃");
        assert_eq!(clean_html_text(""), "");
        assert_eq!(clean_html_text("  "), "");
    }

    #[test]
    fn decodes_entities_via_parser() {
        assert_eq!(clean_html_text("a &amp; b"), "a & b");
        assert_eq!(clean_html_text("&lt;tag&gt;"), "<tag>");
        assert_eq!(clean_html_text("it&#39;s &quot;ok&quot;"), "it's \"ok\"");
        assert_eq!(clean_html_text("&#20320;&#22909;"), "你好");
        assert_eq!(clean_html_text("&#x4F60;&#x597D;"), "你好");
        assert_eq!(clean_html_text("a&nbsp;b"), "a b");
        // 未知实体原样保留
        assert_eq!(clean_html_text("&unknown;"), "&unknown;");
        assert_eq!(clean_html_text("100% & 200"), "100% & 200");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(collapse_ws("  a \n\t b  "), "a b");
        assert_eq!(collapse_ws("\u{3000}a\u{3000}"), "a");
    }

    #[test]
    fn caps_by_character_count() {
        assert_eq!(cap_chars("abcdef", 3), "abc");
        assert_eq!(cap_chars("abc", 3), "abc");
        assert_eq!(cap_chars("放弃抛弃", 2), "放弃");
        assert_eq!(cap_chars("abc", 0), "");
    }

    #[test]
    fn normalizes_phonetic() {
        assert_eq!(
            normalize_phonetic("əˈbændən").as_deref(),
            Some("/əˈbændən/")
        );
        assert_eq!(
            normalize_phonetic("/əˈbændən/").as_deref(),
            Some("/əˈbændən/")
        );
        assert_eq!(
            normalize_phonetic("[əˈbændən]").as_deref(),
            Some("/əˈbændən/")
        );
        assert_eq!(normalize_phonetic("   "), None);
        assert_eq!(normalize_phonetic("[]"), None);
    }

    #[test]
    fn scans_between_markers() {
        let description = "abandon: 美[əˈbændən]，英[əˈbændən]，v. 放弃";
        assert_eq!(between(description, "美[", "]"), Some("əˈbændən"));
        assert_eq!(between(description, "英[", "]"), Some("əˈbændən"));
        assert_eq!(between(description, "法[", "]"), None);
        assert_eq!(between(description, "美[", "（"), None);
    }

    #[test]
    fn encodes_url_segments() {
        assert_eq!(url_encode("abandon"), "abandon");
        assert_eq!(url_encode("a b"), "a%20b");
        assert!(url_encode("it's").contains("%27"));
    }
}
