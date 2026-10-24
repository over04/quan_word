use std::collections::HashMap;

use scraper::{ElementRef, Selector};
use serde::Deserialize;

use super::document::Document;
use super::json_path;
use super::prefer_equals::PreferEquals;
use super::prefer_fallback::PreferFallback;
use super::prefer_spec::PreferSpec;
use super::value_ref::ValueRef;

/// 提取器：从 JSON 文档、HTML 文档、变量或查询词中取值。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Extractor {
    Path {
        path: String,
    },
    All {
        all: String,
    },
    Selector {
        selector: String,
        #[serde(default)]
        attr: Option<String>,
        #[serde(default)]
        all: bool,
    },
    Var {
        var: String,
    },
    Query {
        query: bool,
    },
    Text {
        text: String,
    },
    Template {
        template: String,
    },
    First {
        first: Vec<Extractor>,
    },
    Prefer {
        prefer: PreferSpec,
    },
    Field {
        field: String,
        from: Box<Extractor>,
    },
}

impl Extractor {
    pub fn eval<'a>(
        &self,
        ctx: Option<ValueRef<'a>>,
        doc: &'a Document,
        vars: &HashMap<String, Vec<ValueRef<'a>>>,
        query: &str,
    ) -> Vec<ValueRef<'a>> {
        match self {
            Self::Path { path } => {
                let Some(root) = json_root(ctx.as_ref(), doc) else {
                    return Vec::new();
                };
                let Ok(segments) = json_path::parse(path) else {
                    return Vec::new();
                };
                json_path::evaluate(root, &segments)
                    .into_iter()
                    .take(1)
                    .map(ValueRef::Json)
                    .collect()
            }
            Self::All { all } => {
                let Some(root) = json_root(ctx.as_ref(), doc) else {
                    return Vec::new();
                };
                let Ok(segments) = json_path::parse(all) else {
                    return Vec::new();
                };
                json_path::evaluate(root, &segments)
                    .into_iter()
                    .map(ValueRef::Json)
                    .collect()
            }
            Self::Selector {
                selector,
                attr,
                all,
            } => {
                let Ok(selector) = Selector::parse(selector) else {
                    return Vec::new();
                };
                let elements: Vec<ElementRef<'a>> = match ctx.as_ref() {
                    Some(ValueRef::Html(element)) => element.select(&selector).collect(),
                    _ => match doc.html() {
                        Some(html) => html.select(&selector).collect(),
                        None => return Vec::new(),
                    },
                };
                let mut values = Vec::new();
                for element in elements {
                    if let Some(attr) = attr {
                        if let Some(value) = element.value().attr(attr) {
                            if !value.is_empty() {
                                values.push(ValueRef::Text(value.to_string()));
                            }
                        }
                    } else {
                        values.push(ValueRef::Html(element));
                        if !all {
                            break;
                        }
                    }
                }
                values
            }
            Self::Var { var } => vars.get(var).cloned().unwrap_or_default(),
            Self::Query { query: enabled } => {
                if *enabled {
                    vec![ValueRef::Text(query.to_string())]
                } else {
                    Vec::new()
                }
            }
            Self::Text { text } => vec![ValueRef::Text(text.clone())],
            Self::Template { template } => vec![ValueRef::Text(
                template.replace("{word}", &crate::text::url_encode(query)),
            )],
            Self::First { first } => {
                for extractor in first {
                    for value in extractor.eval(ctx.clone(), doc, vars, query) {
                        let text = value.as_string();
                        if !text.is_empty() {
                            return vec![ValueRef::Text(text)];
                        }
                    }
                }
                Vec::new()
            }
            Self::Prefer { prefer } => {
                let nodes = prefer.from.eval(ctx, doc, vars, query);
                let matched: Vec<ValueRef<'a>> = nodes
                    .iter()
                    .filter(|node| prefer.matches(node, query))
                    .cloned()
                    .collect();
                if !matched.is_empty() {
                    return matched;
                }
                match prefer.fallback {
                    PreferFallback::First => nodes.into_iter().take(1).collect(),
                    PreferFallback::None => Vec::new(),
                }
            }
            Self::Field { field, from } => {
                let nodes = from.eval(ctx, doc, vars, query);
                let Some(node) = nodes.first() else {
                    return Vec::new();
                };
                match node {
                    ValueRef::Json(value) => value
                        .get(field)
                        .map(|value| vec![ValueRef::Json(value)])
                        .unwrap_or_default(),
                    ValueRef::Html(element) => element
                        .value()
                        .attr(field)
                        .map(|value| vec![ValueRef::Text(value.to_string())])
                        .unwrap_or_default(),
                    ValueRef::Text(_) => Vec::new(),
                }
            }
        }
    }
}

impl PreferSpec {
    fn matches(&self, node: &ValueRef<'_>, query: &str) -> bool {
        if !matches!(self.equals, PreferEquals::Query) {
            return false;
        }
        let Some(actual) = node
            .as_json()
            .and_then(|value| value.get(&self.field))
            .and_then(|value| value.as_str())
        else {
            return false;
        };
        if self.ignore_case {
            actual.eq_ignore_ascii_case(query)
        } else {
            actual == query
        }
    }
}

fn json_root<'a>(ctx: Option<&ValueRef<'a>>, doc: &'a Document) -> Option<&'a serde_json::Value> {
    match ctx {
        Some(ValueRef::Json(value)) => Some(value),
        _ => doc.json(),
    }
}
