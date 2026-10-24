use serde_json::Value;

use crate::plugin_error::PluginError;

/// JSON 路径片段：对象键、数组下标、数组展开。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
    All,
}

/// 解析 `a.b[0].c[*]` 形式的路径；不支持过滤器与表达式。
pub fn parse(raw: &str) -> Result<Vec<PathSegment>, PluginError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(PluginError::InvalidPath("路径不能为空".into()));
    }
    let mut segments = Vec::new();
    let mut name = String::new();
    let mut expect_segment = true;
    let mut last_was_bracket = false;
    let mut chars = trimmed.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if expect_segment {
                    return Err(PluginError::InvalidPath(format!("空片段: {raw}")));
                }
                if !name.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut name)));
                }
                expect_segment = true;
                last_was_bracket = false;
            }
            '[' => {
                if !name.is_empty() {
                    segments.push(PathSegment::Key(std::mem::take(&mut name)));
                }
                let mut inner = String::new();
                loop {
                    match chars.next() {
                        Some(']') => break,
                        Some(c) => inner.push(c),
                        None => {
                            return Err(PluginError::InvalidPath(format!("缺少 ]: {raw}")));
                        }
                    }
                }
                if inner == "*" {
                    segments.push(PathSegment::All);
                } else {
                    let index: usize = inner
                        .parse()
                        .map_err(|_| PluginError::InvalidPath(format!("下标非法: {raw}")))?;
                    segments.push(PathSegment::Index(index));
                }
                expect_segment = false;
                last_was_bracket = true;
            }
            c => {
                name.push(c);
                expect_segment = false;
                last_was_bracket = false;
            }
        }
    }
    if name.is_empty() {
        if expect_segment && !last_was_bracket {
            return Err(PluginError::InvalidPath(format!("路径不完整: {raw}")));
        }
    } else {
        segments.push(PathSegment::Key(name));
    }
    if segments.is_empty() {
        return Err(PluginError::InvalidPath(format!("路径为空: {raw}")));
    }
    Ok(segments)
}

/// 按路径求值；`[*]` 对标量返回自身（兼容上游「字符串或数组」字段）。
pub fn evaluate<'a>(root: &'a Value, segments: &[PathSegment]) -> Vec<&'a Value> {
    let mut current = vec![root];
    for segment in segments {
        let mut next = Vec::new();
        for value in current {
            match segment {
                PathSegment::Key(key) => {
                    if let Some(child) = value.get(key) {
                        next.push(child);
                    }
                }
                PathSegment::Index(index) => {
                    if let Some(child) = value.get(*index) {
                        next.push(child);
                    }
                }
                PathSegment::All => match value {
                    Value::Array(items) => next.extend(items.iter()),
                    other => next.push(other),
                },
            }
        }
        current = next;
    }
    current
}
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{evaluate, parse};

    #[test]
    fn parses_nested_paths_and_arrays() {
        let segments = parse("ec.word[0].trs[*].tr[*].l.i[*]").unwrap();
        let value = json!({
            "ec": {"word": [{"trs": [{"tr": [{"l": {"i": ["a", "b"]}}]}]}]}
        });
        let found = evaluate(&value, &segments);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], "a");
        assert_eq!(found[1], "b");
    }

    #[test]
    fn supports_root_array_and_scalar_all() {
        let value = json!([{"text": "x"}, {"text": "y"}]);
        let found = evaluate(&value, &parse("[*].text").unwrap());
        assert_eq!(found.len(), 2);
        let scalar = json!({"v": "only"});
        let found = evaluate(&scalar, &parse("v[*]").unwrap());
        assert_eq!(found, vec![&json!("only")]);
    }

    #[test]
    fn rejects_invalid_paths() {
        for raw in ["", "a..b", "a[", "a[x]", "a[]"] {
            assert!(parse(raw).is_err(), "{raw}");
        }
    }
}
