use scraper::Selector;
use url::Url;

use crate::plugin_error::PluginError;

use super::condition::Condition;
use super::definition_strategy::DefinitionStrategy;
use super::extractor::Extractor;
use super::items_spec::ItemsSpec;
use super::lookup_recipe::LookupRecipe;
use super::suggest_recipe::SuggestRecipe;
use super::transform::Transform;
use super::Recipe;

/// 主机模式：`host` / `*.suffix` / `host:port`。
pub fn validate_host_pattern(raw: &str) -> Result<(), PluginError> {
    if raw.is_empty()
        || raw.contains('/')
        || raw.contains("://")
        || raw.chars().any(char::is_whitespace)
    {
        return Err(PluginError::InvalidHost(raw.to_string()));
    }
    let host = raw.strip_prefix("*.").unwrap_or(raw);
    let (host, port) = match host.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => (host, Some(port)),
        _ => (host, None),
    };
    if host.is_empty()
        || !host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(PluginError::InvalidHost(raw.to_string()));
    }
    if let Some(port) = port {
        port.parse::<u16>()
            .map_err(|_| PluginError::InvalidHost(raw.to_string()))?;
    }
    Ok(())
}

/// URL 主机是否命中白名单；带端口的模式同时比较端口。
pub fn host_allowed(host: &str, port: Option<u16>, hosts: &[String]) -> bool {
    hosts.iter().any(|pattern| {
        let (pattern_host, pattern_port) = split_host_port(pattern);
        let matched = if let Some(suffix) = pattern_host.strip_prefix("*.") {
            host.ends_with(&format!(".{suffix}"))
        } else {
            host == pattern_host
        };
        matched && pattern_port.is_none_or(|expected| Some(expected) == port)
    })
}

fn split_host_port(pattern: &str) -> (&str, Option<u16>) {
    match pattern.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => {
            (host, port.parse::<u16>().ok())
        }
        _ => (pattern, None),
    }
}

/// 请求头名称：ASCII 字母数字与连字符。
pub fn validate_header_name(name: &str) -> Result<(), PluginError> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(PluginError::InvalidRecipe(format!(
            "请求头名称非法: {name}"
        )));
    }
    Ok(())
}

impl Extractor {
    pub fn validate(&self) -> Result<(), PluginError> {
        match self {
            Self::Path { path } | Self::All { all: path } => {
                super::json_path::parse(path)?;
            }
            Self::Selector { selector, attr, .. } => {
                Selector::parse(selector)
                    .map_err(|e| PluginError::InvalidSelector(format!("{selector}: {e}")))?;
                if attr.as_deref().is_some_and(str::is_empty) {
                    return Err(PluginError::InvalidRecipe("attr 不能为空".into()));
                }
            }
            Self::Var { var } => {
                if var.trim().is_empty() {
                    return Err(PluginError::InvalidRecipe("变量名不能为空".into()));
                }
            }
            Self::Query { query } => {
                if !query {
                    return Err(PluginError::InvalidRecipe("query 只能为 true".into()));
                }
            }
            Self::Text { .. } => {}
            Self::Template { template } => validate_template(template)?,
            Self::First { first } => {
                if first.is_empty() {
                    return Err(PluginError::InvalidRecipe("first 不能为空".into()));
                }
                for extractor in first {
                    extractor.validate()?;
                }
            }
            Self::Prefer { prefer } => {
                if prefer.field.trim().is_empty() {
                    return Err(PluginError::InvalidRecipe("prefer.field 不能为空".into()));
                }
                prefer.from.validate()?;
            }
            Self::Field { field, from } => {
                if field.trim().is_empty() {
                    return Err(PluginError::InvalidRecipe("field 不能为空".into()));
                }
                from.validate()?;
            }
        }
        Ok(())
    }
}

impl Transform {
    pub fn validate(&self) -> Result<(), PluginError> {
        match self {
            Self::Unit(_) => {}
            Self::Join { .. } => {}
            Self::Split { split } => validate_separators(split)?,
            Self::Take { take } => {
                if *take == 0 {
                    return Err(PluginError::InvalidRecipe("take 必须大于 0".into()));
                }
            }
            Self::Dedup { dedup } => {
                if dedup != "lowercase" {
                    return Err(PluginError::InvalidRecipe(format!(
                        "不支持的 dedup: {dedup}"
                    )));
                }
            }
            Self::StripChars { strip_chars } => {
                if strip_chars.is_empty() {
                    return Err(PluginError::InvalidRecipe("strip_chars 不能为空".into()));
                }
            }
            Self::Between { between } => {
                if between[0].is_empty() || between[1].is_empty() {
                    return Err(PluginError::InvalidRecipe(
                        "between 的前后缀不能为空".into(),
                    ));
                }
            }
            Self::Before { before } => {
                if before.is_empty() {
                    return Err(PluginError::InvalidRecipe("before 不能为空".into()));
                }
            }
            Self::After { after } => {
                if after.is_empty() {
                    return Err(PluginError::InvalidRecipe("after 不能为空".into()));
                }
            }
            Self::AfterLast { after_last } => {
                if after_last.is_empty() {
                    return Err(PluginError::InvalidRecipe("after_last 不能为空".into()));
                }
            }
            Self::TrimChars { trim_chars } => {
                if trim_chars.is_empty() {
                    return Err(PluginError::InvalidRecipe("trim_chars 不能为空".into()));
                }
            }
            Self::CapChars { cap_chars } => {
                if *cap_chars == 0 {
                    return Err(PluginError::InvalidRecipe("cap_chars 必须大于 0".into()));
                }
            }
            Self::Default { .. } => {}
        }
        Ok(())
    }
}

impl Condition {
    pub fn validate(&self) -> Result<(), PluginError> {
        let name = match self {
            Self::NonEmpty { non_empty } => non_empty,
            Self::Empty { empty } => empty,
            Self::Contains { contains } => {
                if contains.var.trim().is_empty() {
                    return Err(PluginError::InvalidRecipe("contains.var 不能为空".into()));
                }
                &contains.var
            }
        };
        if name.trim().is_empty() {
            return Err(PluginError::InvalidRecipe("条件变量名不能为空".into()));
        }
        Ok(())
    }
}

impl ItemsSpec {
    pub fn validate(&self, required: &[&str]) -> Result<(), PluginError> {
        self.from.validate()?;
        for name in required {
            if !self.fields.contains_key(*name) {
                return Err(PluginError::InvalidRecipe(format!(
                    "items 缺少字段: {name}"
                )));
            }
        }
        for extractor in self.fields.values() {
            extractor.validate()?;
        }
        if let Some(take) = self.take {
            if take == 0 {
                return Err(PluginError::InvalidRecipe("take 必须大于 0".into()));
            }
        }
        for transform in &self.transform {
            transform.validate()?;
        }
        Ok(())
    }
}

impl DefinitionStrategy {
    pub fn validate(&self) -> Result<(), PluginError> {
        match self {
            Self::PosText {
                from,
                separators,
                when,
                transform,
            }
            | Self::PlainText {
                from,
                separators,
                when,
                transform,
            } => {
                from.validate()?;
                validate_separators(separators)?;
                if let Some(when) = when {
                    when.validate()?;
                }
                for step in transform {
                    step.validate()?;
                }
            }
            Self::Items {
                from,
                pos,
                meaning,
                split,
                when,
                pos_transform,
                transform,
            } => {
                from.validate()?;
                if let Some(pos) = pos {
                    pos.validate()?;
                }
                meaning.validate()?;
                if !split.is_empty() {
                    validate_separators(split)?;
                }
                if let Some(when) = when {
                    when.validate()?;
                }
                for step in pos_transform.iter().chain(transform.iter()) {
                    step.validate()?;
                }
            }
            Self::Groups {
                from,
                pos,
                items,
                when,
                pos_transform,
                transform,
            } => {
                from.validate()?;
                if let Some(pos) = pos {
                    pos.validate()?;
                }
                items.validate()?;
                if let Some(when) = when {
                    when.validate()?;
                }
                for step in pos_transform.iter().chain(transform.iter()) {
                    step.validate()?;
                }
            }
        }
        Ok(())
    }
}

impl Recipe {
    pub fn validate(&self, hosts: &[String], max_body_bytes: usize) -> Result<(), PluginError> {
        if self.lookup.is_none() && self.suggest.is_none() {
            return Err(PluginError::InvalidRecipe(
                "配方至少需要 lookup 或 suggest".into(),
            ));
        }
        if let Some(lookup) = &self.lookup {
            lookup.validate(hosts, max_body_bytes)?;
        }
        if let Some(suggest) = &self.suggest {
            suggest.validate(hosts, max_body_bytes)?;
        }
        Ok(())
    }
}

impl LookupRecipe {
    pub fn validate(&self, hosts: &[String], max_body_bytes: usize) -> Result<(), PluginError> {
        self.request.validate(hosts, max_body_bytes)?;
        let mut names: Vec<&str> = Vec::new();
        for var in &self.vars {
            if var.name.trim().is_empty() {
                return Err(PluginError::InvalidRecipe("变量名不能为空".into()));
            }
            if names.contains(&var.name.as_str()) {
                return Err(PluginError::InvalidRecipe(format!(
                    "变量名重复: {}",
                    var.name
                )));
            }
            names.push(&var.name);
            var.extractor.validate()?;
            for transform in &var.transform {
                transform.validate()?;
            }
        }
        for extractor in [
            self.word.as_ref(),
            self.phonetic.as_ref(),
            self.audio_url.as_ref(),
            self.source_url.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            extractor.validate()?;
        }
        if let Some(Extractor::Template { template }) = &self.source_url {
            validate_template(template)?;
        }
        if self.definitions.is_empty() {
            return Err(PluginError::InvalidRecipe(
                "lookup 至少需要一条 definitions 策略".into(),
            ));
        }
        for strategy in &self.definitions {
            strategy.validate()?;
        }
        if let Some(examples) = &self.examples {
            examples.validate(&["text"])?;
        }
        Ok(())
    }
}

impl SuggestRecipe {
    pub fn validate(&self, hosts: &[String], max_body_bytes: usize) -> Result<(), PluginError> {
        self.request.validate(hosts, max_body_bytes)?;
        self.items.validate(&["word", "summary"])?;
        if let Some(take) = self.take {
            if take == 0 {
                return Err(PluginError::InvalidRecipe("take 必须大于 0".into()));
            }
        }
        Ok(())
    }
}

fn validate_separators(separators: &[String]) -> Result<(), PluginError> {
    if separators.is_empty() {
        return Err(PluginError::InvalidRecipe("separators 不能为空".into()));
    }
    for separator in separators {
        if separator.chars().count() != 1 {
            return Err(PluginError::InvalidRecipe(format!(
                "separator 必须是单字符: {separator}"
            )));
        }
    }
    Ok(())
}

fn validate_template(template: &str) -> Result<(), PluginError> {
    let rendered = template.replace("{word}", "word");
    if rendered.contains('{') || rendered.contains('}') {
        return Err(PluginError::InvalidRecipe(format!(
            "未知占位符: {template}"
        )));
    }
    if template.starts_with("http://") || template.starts_with("https://") {
        let _ = Url::parse(&rendered)
            .map_err(|e| PluginError::InvalidUrl(format!("{template}: {e}")))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::{host_allowed, validate_host_pattern};

    #[test]
    fn validates_host_patterns() {
        for raw in ["dict.youdao.com", "*.example.com", "localhost:8080"] {
            assert!(validate_host_pattern(raw).is_ok(), "{raw}");
        }
        for raw in ["", "https://x.com", "x/y", "x y", ":80"] {
            assert!(validate_host_pattern(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn matches_exact_wildcard_and_port() {
        let hosts = vec![
            "dict.youdao.com".to_string(),
            "*.example.com".to_string(),
            "localhost:8080".to_string(),
        ];
        assert!(host_allowed("dict.youdao.com", Some(443), &hosts));
        assert!(!host_allowed("other.youdao.com", Some(443), &hosts));
        assert!(host_allowed("a.example.com", Some(443), &hosts));
        assert!(!host_allowed("example.com", Some(443), &hosts));
        assert!(host_allowed("localhost", Some(8080), &hosts));
        assert!(!host_allowed("localhost", Some(80), &hosts));
    }
}
