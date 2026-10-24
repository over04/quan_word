use std::time::Duration;

use async_trait::async_trait;
use dict_core::form_field_spec::FormFieldSpec;
use dict_core::header_spec::HeaderSpec;
use dict_core::recipe::validate::host_allowed;
use dict_core::request_method::RequestMethod;
use dict_core::search_request::SearchRequest;
use dict_core::search_response::SearchResponse;
use dict_core::transport::SearchTransport;
use dict_core::transport_error::TransportError;
use reqwest::header::LOCATION;
use reqwest::{Client, Response, StatusCode, Url};

/// 固定 Chrome UA：部分词典站点对非浏览器 UA 直接返回空页。
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// 最多跟随的重定向次数（与旧实现 `Policy::limited(3)` 一致）。
const MAX_REDIRECTS: usize = 3;

/// 服务器端传输层：统一超时、UA、手动重定向校验与响应体上限。
#[derive(Clone)]
pub struct ReqwestTransport {
    client: Client,
}

impl ReqwestTransport {
    pub fn new(timeout_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|error| {
                tracing::error!("构造词典 HTTP 客户端失败，回退默认客户端: {error}");
                Client::new()
            });
        Self { client }
    }

    async fn send_once(
        &self,
        method: RequestMethod,
        url: &str,
        headers: &[HeaderSpec],
        form: &[FormFieldSpec],
    ) -> Result<Response, TransportError> {
        let mut request = match method {
            RequestMethod::Get => self.client.get(url),
            RequestMethod::PostForm => {
                let fields: Vec<(&str, &str)> = form
                    .iter()
                    .map(|field| (field.name.as_str(), field.value.as_str()))
                    .collect();
                self.client.post(url).form(&fields)
            }
        };
        for header in headers {
            request = request.header(header.name.as_str(), header.value.as_str());
        }
        request.send().await.map_err(map_reqwest)
    }
}

#[async_trait]
impl SearchTransport for ReqwestTransport {
    async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError> {
        let mut url = Url::parse(&request.url)
            .map_err(|error| TransportError::Http(format!("URL 非法: {error}")))?;
        let mut method = request.method;
        let mut form = request.form.clone();
        for _ in 0..=MAX_REDIRECTS {
            let response = self
                .send_once(method, url.as_str(), &request.headers, &form)
                .await?;
            if response.status().is_redirection() {
                let Some(location) = response.headers().get(LOCATION) else {
                    return Err(TransportError::Http(format!("HTTP {}", response.status())));
                };
                let location = location
                    .to_str()
                    .map_err(|error| TransportError::Http(format!("重定向地址非法: {error}")))?;
                let next = url
                    .join(location)
                    .map_err(|error| TransportError::Http(format!("重定向地址非法: {error}")))?;
                if !host_allowed(
                    next.host_str().unwrap_or_default(),
                    next.port_or_known_default(),
                    &request.allowed_hosts,
                ) {
                    return Err(TransportError::RedirectBlocked(
                        next.host_str().unwrap_or_default().to_string(),
                    ));
                }
                // 303 与 301/302 的 POST 按浏览器语义改成 GET；307/308 保留表单。
                if !matches!(
                    response.status(),
                    StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT
                ) && method == RequestMethod::PostForm
                {
                    method = RequestMethod::Get;
                    form.clear();
                }
                url = next;
                continue;
            }
            let final_url = url.to_string();
            let status = response.status();
            if status == StatusCode::NOT_FOUND {
                return Err(TransportError::NotFound);
            }
            if !status.is_success() {
                return Err(TransportError::Http(format!("HTTP {status}")));
            }
            let body = read_text(response, request.max_body_bytes).await?;
            return Ok(SearchResponse {
                status: status.as_u16(),
                body,
                final_url,
            });
        }
        Err(TransportError::Http("重定向次数过多".into()))
    }
}

async fn read_text(
    mut response: Response,
    max_body_bytes: usize,
) -> Result<String, TransportError> {
    let mut buffer: Vec<u8> = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_reqwest)? {
        if buffer.len() + chunk.len() > max_body_bytes {
            return Err(TransportError::TooLarge);
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

fn map_reqwest(error: reqwest::Error) -> TransportError {
    if error.is_timeout() {
        TransportError::Timeout
    } else {
        TransportError::Http(error.to_string())
    }
}
