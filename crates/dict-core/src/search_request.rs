use crate::form_field_spec::FormFieldSpec;
use crate::header_spec::HeaderSpec;
use crate::request_method::RequestMethod;

/// 交给宿主传输层的一次请求。
#[derive(Debug, Clone)]
pub struct SearchRequest {
    pub method: RequestMethod,
    pub url: String,
    pub headers: Vec<HeaderSpec>,
    pub form: Vec<FormFieldSpec>,
    /// 允许访问的主机（manifest.hosts），传输层还要校验重定向终点。
    pub allowed_hosts: Vec<String>,
    pub max_body_bytes: usize,
}
