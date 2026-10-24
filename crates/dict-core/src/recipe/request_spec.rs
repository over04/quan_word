use serde::Deserialize;
use url::Url;

use crate::form_field_spec::FormFieldSpec;
use crate::header_spec::HeaderSpec;
use crate::plugin_error::PluginError;
use crate::provider_error::DictProviderError;
use crate::request_method::RequestMethod;
use crate::search_request::SearchRequest;
use crate::text::url_encode;

use super::validate::{host_allowed, validate_header_name};

/// 一次上游请求的声明：方法、URL 模板、请求头、表单字段。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestSpec {
    pub method: RequestMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HeaderSpec>,
    #[serde(default)]
    pub form: Vec<FormFieldSpec>,
}

impl RequestSpec {
    /// 装载期校验：URL 合法、host 在清单内、占位符与请求头安全。
    pub fn validate(&self, hosts: &[String], max_body_bytes: usize) -> Result<(), PluginError> {
        let rendered = render_template(&self.url, "word")?;
        let url = Url::parse(&rendered)
            .map_err(|e| PluginError::InvalidUrl(format!("{}: {e}", self.url)))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(PluginError::InvalidUrl(format!(
                "只允许 http/https: {}",
                self.url
            )));
        }
        let host = url
            .host_str()
            .ok_or_else(|| PluginError::InvalidUrl(format!("缺少 host: {}", self.url)))?;
        if !host_allowed(host, url.port_or_known_default(), hosts) {
            return Err(PluginError::HostNotAllowed(host.to_string()));
        }
        for header in &self.headers {
            validate_header_name(&header.name)?;
            if header.value.contains('\r')
                || header.value.contains('\n')
                || header.value.contains('{')
            {
                return Err(PluginError::InvalidRecipe(format!(
                    "请求头 {} 的值非法",
                    header.name
                )));
            }
        }
        for field in &self.form {
            if field.name.trim().is_empty() {
                return Err(PluginError::InvalidRecipe("表单字段名不能为空".into()));
            }
            let _ = render_template(&field.value, "word")?;
        }
        if max_body_bytes == 0 {
            return Err(PluginError::InvalidRecipe("响应上限必须大于 0".into()));
        }
        Ok(())
    }

    /// 运行时构造请求（`{word}` 按 URL 编码替换）。
    pub fn build(
        &self,
        word: &str,
        hosts: &[String],
        max_body_bytes: usize,
    ) -> Result<SearchRequest, DictProviderError> {
        let url = self.url.replace("{word}", &url_encode(word));
        if url.contains('{') || url.contains('}') {
            return Err(DictProviderError::Parse(format!(
                "插件 URL 含未知占位符: {}",
                self.url
            )));
        }
        Ok(SearchRequest {
            method: self.method,
            url,
            headers: self.headers.clone(),
            form: self
                .form
                .iter()
                .map(|field| FormFieldSpec {
                    name: field.name.clone(),
                    value: field.value.replace("{word}", &url_encode(word)),
                })
                .collect(),
            allowed_hosts: hosts.to_vec(),
            max_body_bytes,
        })
    }
}

/// 把模板里的占位符替换为给定值；发现其它 `{...}` 直接报错。
fn render_template(template: &str, word: &str) -> Result<String, PluginError> {
    let rendered = template.replace("{word}", word);
    if rendered.contains('{') || rendered.contains('}') {
        return Err(PluginError::InvalidUrl(format!("未知占位符: {template}")));
    }
    Ok(rendered)
}
