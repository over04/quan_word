use std::collections::HashMap;

use lexicon::definition::Definition;

use crate::engine_limits::EngineLimits;
use crate::entry::DictEntry;
use crate::example::DictExample;
use crate::plugin_info::PluginInfo;
use crate::pos::{add_definition, pos_definitions};
use crate::provider_error::DictProviderError;
use crate::suggestion::DictSuggestion;
use crate::text::{clean_html_text, collapse_ws, normalize_phonetic, url_encode};

use super::definition_strategy::DefinitionStrategy;
use super::document::Document;
use super::empty_behavior::EmptyBehavior;
use super::extractor::Extractor;
use super::items_spec::ItemsSpec;
use super::lookup_recipe::LookupRecipe;
use super::suggest_recipe::SuggestRecipe;
use super::transform::Transform;
use super::unit_transform::UnitTransform;
use super::value_ref::ValueRef;

impl LookupRecipe {
    /// 执行抓词配方；释义为空视为上游未收录。
    pub fn execute(
        &self,
        body: &str,
        query: &str,
        info: &PluginInfo,
        limits: &EngineLimits,
    ) -> Result<DictEntry, DictProviderError> {
        let doc = Document::detect(body);
        let mut vars: HashMap<String, Vec<ValueRef<'_>>> = HashMap::new();
        for var in &self.vars {
            let values = var.extractor.eval(None, &doc, &vars, query);
            let values = apply_transforms(values, &var.transform);
            vars.insert(var.name.clone(), values);
        }

        let word = self
            .word
            .as_ref()
            .and_then(|extractor| first_string(extractor.eval(None, &doc, &vars, query)))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| query.to_string());
        let phonetic = self
            .phonetic
            .as_ref()
            .and_then(|extractor| first_string(extractor.eval(None, &doc, &vars, query)))
            .filter(|value| !value.is_empty());
        let audio_url = self
            .audio_url
            .as_ref()
            .and_then(|extractor| first_string(extractor.eval(None, &doc, &vars, query)))
            .filter(|value| !value.is_empty());
        let source_url = self
            .source_url
            .as_ref()
            .and_then(|extractor| first_string(extractor.eval(None, &doc, &vars, query)))
            .filter(|value| !value.is_empty());

        let definitions = self.build_definitions(&doc, &vars, query, limits);
        if definitions.is_empty() {
            return Err(match self.empty {
                EmptyBehavior::NotFound => DictProviderError::NotFound,
                EmptyBehavior::Parse => DictProviderError::Parse("页面未解析到释义".to_string()),
            });
        }
        let examples = self.build_examples(&doc, &vars, query, limits);

        Ok(DictEntry {
            provider: info.id.clone(),
            label: info.label.clone(),
            word,
            phonetic,
            definitions,
            examples,
            audio_url,
            source_url,
        })
    }

    fn build_definitions<'a>(
        &self,
        doc: &'a Document,
        vars: &HashMap<String, Vec<ValueRef<'a>>>,
        query: &str,
        limits: &EngineLimits,
    ) -> Vec<Definition> {
        let limit = limits.max_definitions;
        for strategy in &self.definitions {
            if !strategy_allowed(strategy, vars) {
                continue;
            }
            let definitions = match strategy {
                DefinitionStrategy::PosText {
                    from,
                    separators,
                    transform,
                    ..
                } => {
                    let text = joined_text(from, transform, doc, vars, query);
                    pos_definitions(&text, &separator_chars(separators))
                }
                DefinitionStrategy::PlainText {
                    from,
                    separators,
                    transform,
                    ..
                } => {
                    let text = joined_text(from, transform, doc, vars, query);
                    let separators = separator_chars(separators);
                    let mut definitions = Vec::new();
                    for piece in text.split(|c: char| separators.contains(&c)) {
                        add_definition(&mut definitions, "", piece);
                    }
                    definitions
                }
                DefinitionStrategy::Items {
                    from,
                    pos,
                    meaning,
                    split,
                    pos_transform,
                    transform,
                    ..
                } => {
                    let mut definitions = Vec::new();
                    for node in from.eval(None, doc, vars, query) {
                        let pos = pos_text(pos, pos_transform, node.clone(), doc, vars, query);
                        let meaning = field_text(meaning, transform, node, doc, vars, query);
                        if split.is_empty() {
                            add_definition(&mut definitions, &pos, &meaning);
                        } else {
                            let separators = separator_chars(split);
                            for piece in meaning.split(|c: char| separators.contains(&c)) {
                                add_definition(&mut definitions, &pos, piece);
                            }
                        }
                    }
                    definitions
                }
                DefinitionStrategy::Groups {
                    from,
                    pos,
                    items,
                    pos_transform,
                    transform,
                    ..
                } => {
                    let mut definitions = Vec::new();
                    for node in from.eval(None, doc, vars, query) {
                        let pos = pos_text(pos, pos_transform, node.clone(), doc, vars, query);
                        for item in
                            apply_transforms(items.eval(Some(node), doc, vars, query), transform)
                        {
                            add_definition(&mut definitions, &pos, &item.as_string());
                        }
                    }
                    definitions
                }
            };
            if !definitions.is_empty() {
                return definitions.into_iter().take(limit).collect();
            }
        }
        Vec::new()
    }

    fn build_examples<'a>(
        &self,
        doc: &'a Document,
        vars: &HashMap<String, Vec<ValueRef<'a>>>,
        query: &str,
        limits: &EngineLimits,
    ) -> Vec<DictExample> {
        let Some(spec) = &self.examples else {
            return Vec::new();
        };
        let limit = spec
            .take
            .unwrap_or(limits.max_examples)
            .min(limits.max_examples);
        let mut examples = Vec::new();
        for node in spec.from.eval(None, doc, vars, query) {
            let text = item_field(spec, "text", node.clone(), doc, vars, query);
            if text.is_empty() {
                continue;
            }
            let translation = item_field(spec, "translation", node, doc, vars, query);
            examples.push(DictExample {
                text,
                translation: if translation.is_empty() {
                    None
                } else {
                    Some(translation)
                },
            });
            if examples.len() >= limit {
                break;
            }
        }
        examples
    }
}

impl SuggestRecipe {
    /// 执行建议配方；失败/空结果由上层决定是否继续下一个源。
    pub fn execute(
        &self,
        body: &str,
        query: &str,
        limits: &EngineLimits,
    ) -> Result<Vec<DictSuggestion>, DictProviderError> {
        let doc = Document::detect(body);
        let vars: HashMap<String, Vec<ValueRef<'_>>> = HashMap::new();
        let take = self
            .take
            .unwrap_or(limits.max_suggestions)
            .min(limits.max_suggestions);
        let mut suggestions: Vec<DictSuggestion> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for node in self.items.from.eval(None, &doc, &vars, query) {
            let word = item_field(&self.items, "word", node.clone(), &doc, &vars, query);
            let summary = item_field(&self.items, "summary", node, &doc, &vars, query);
            if word.is_empty() || summary.is_empty() {
                continue;
            }
            if matches!(
                self.dedup,
                Some(super::dedup_mode::DedupMode::WordLowercase)
            ) {
                let key = word.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key);
            }
            suggestions.push(DictSuggestion { word, summary });
            if suggestions.len() >= take {
                break;
            }
        }
        Ok(suggestions)
    }
}

fn strategy_allowed<'a>(
    strategy: &DefinitionStrategy,
    vars: &HashMap<String, Vec<ValueRef<'a>>>,
) -> bool {
    let condition = match strategy {
        DefinitionStrategy::PosText { when, .. }
        | DefinitionStrategy::PlainText { when, .. }
        | DefinitionStrategy::Items { when, .. }
        | DefinitionStrategy::Groups { when, .. } => when.as_ref(),
    };
    condition.is_none_or(|condition| condition.matches(vars))
}

fn joined_text<'a>(
    from: &Extractor,
    transform: &[Transform],
    doc: &'a Document,
    vars: &HashMap<String, Vec<ValueRef<'a>>>,
    query: &str,
) -> String {
    let values = apply_transforms(from.eval(None, doc, vars, query), transform);
    values
        .iter()
        .map(ValueRef::as_string)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn field_text<'a>(
    extractor: &Extractor,
    transform: &[Transform],
    node: ValueRef<'a>,
    doc: &'a Document,
    vars: &HashMap<String, Vec<ValueRef<'a>>>,
    query: &str,
) -> String {
    let values = apply_transforms(extractor.eval(Some(node), doc, vars, query), transform);
    values
        .iter()
        .map(ValueRef::as_string)
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

fn pos_text<'a>(
    pos: &Option<Extractor>,
    transform: &[Transform],
    node: ValueRef<'a>,
    doc: &'a Document,
    vars: &HashMap<String, Vec<ValueRef<'a>>>,
    query: &str,
) -> String {
    let Some(pos) = pos else {
        return String::new();
    };
    field_text(pos, transform, node, doc, vars, query)
}

fn item_field<'a>(
    spec: &ItemsSpec,
    name: &str,
    node: ValueRef<'a>,
    doc: &'a Document,
    vars: &HashMap<String, Vec<ValueRef<'a>>>,
    query: &str,
) -> String {
    let Some(extractor) = spec.fields.get(name) else {
        return String::new();
    };
    field_text(extractor, &spec.transform, node, doc, vars, query)
}

fn first_string<'a>(values: Vec<ValueRef<'a>>) -> Option<String> {
    values
        .iter()
        .map(ValueRef::as_string)
        .find(|value| !value.is_empty())
}

fn separator_chars(separators: &[String]) -> Vec<char> {
    separators
        .iter()
        .filter_map(|separator| separator.chars().next())
        .collect()
}

fn apply_transforms<'a>(values: Vec<ValueRef<'a>>, transforms: &[Transform]) -> Vec<ValueRef<'a>> {
    let mut values = values;
    for transform in transforms {
        if let Transform::Take { take } = transform {
            values.truncate(*take);
            continue;
        }
        let strings: Vec<String> = values
            .iter()
            .map(ValueRef::as_string)
            .filter(|value| !value.is_empty())
            .collect();
        let strings = apply_string_transform(strings, transform);
        values = strings.into_iter().map(ValueRef::Text).collect();
    }
    values
}

fn apply_string_transform(strings: Vec<String>, transform: &Transform) -> Vec<String> {
    match transform {
        Transform::Unit(unit) => strings
            .into_iter()
            .filter_map(|value| apply_unit(unit, value))
            .collect(),
        Transform::Join { join } => vec![strings.join(join)],
        Transform::Split { split } => {
            let separators: Vec<char> = split
                .iter()
                .filter_map(|separator| separator.chars().next())
                .collect();
            strings
                .into_iter()
                .flat_map(|value| {
                    value
                        .split(|c: char| separators.contains(&c))
                        .map(str::trim)
                        .filter(|piece| !piece.is_empty())
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .collect()
        }
        Transform::Take { take } => strings.into_iter().take(*take).collect(),
        Transform::Dedup { .. } => {
            let mut seen: Vec<String> = Vec::new();
            let mut output = Vec::new();
            for value in strings {
                let key = value.to_lowercase();
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key);
                output.push(value);
            }
            output
        }
        Transform::StripChars { strip_chars } => strings
            .into_iter()
            .map(|value| {
                value
                    .chars()
                    .filter(|c| !strip_chars.iter().any(|chars| chars.contains(*c)))
                    .collect()
            })
            .collect(),
        Transform::Between { between } => strings
            .into_iter()
            .filter_map(|value| {
                crate::text::between(&value, &between[0], &between[1]).map(str::to_string)
            })
            .collect(),
        Transform::Before { before } => strings
            .into_iter()
            .map(|value| {
                value
                    .split_once(before)
                    .map_or_else(|| value.clone(), |(head, _)| head.to_string())
            })
            .collect(),
        Transform::After { after } => strings
            .into_iter()
            .filter_map(|value| value.split_once(after).map(|(_, tail)| tail.to_string()))
            .collect(),
        Transform::AfterLast { after_last } => strings
            .into_iter()
            .map(|value| {
                value
                    .rsplit_once(after_last)
                    .map_or_else(|| value.clone(), |(_, tail)| tail.to_string())
            })
            .collect(),
        Transform::TrimChars { trim_chars } => strings
            .into_iter()
            .map(|value| {
                value
                    .trim_matches(|c: char| trim_chars.contains(c))
                    .to_string()
            })
            .collect(),
        Transform::CapChars { cap_chars } => strings
            .into_iter()
            .map(|value| cap_chars_str(&value, *cap_chars))
            .collect(),
        Transform::Default { default } => {
            if strings.is_empty() {
                vec![default.clone()]
            } else {
                strings
            }
        }
    }
}

fn apply_unit(unit: &UnitTransform, value: String) -> Option<String> {
    Some(match unit {
        UnitTransform::CleanHtml => clean_html_text(&value),
        UnitTransform::CollapseWs => collapse_ws(&value),
        UnitTransform::Trim => value.trim().to_string(),
        UnitTransform::NormalizePhonetic => normalize_phonetic(&value)?,
        UnitTransform::MapEnglishPos => crate::pos::map_english_pos(&value)?.to_string(),
        UnitTransform::CanonicalPos => crate::pos::canonical_pos(&value)?.to_string(),
        UnitTransform::Lowercase => value.to_lowercase(),
        UnitTransform::UrlEncode => url_encode(&value),
    })
}

fn cap_chars_str(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect()
    }
}
