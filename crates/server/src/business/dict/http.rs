use std::time::Duration;

use reqwest::{Client, Response, StatusCode};

use super::form_body::FormBody;
use super::http_header::HttpHeader;
use super::provider_error::DictProviderError;

/// 单个响应的最大字节数（1 MiB）：超出即中止，避免异常上游拖垮进程。
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// 固定 Chrome UA：部分词典站点对非浏览器 UA 直接返回空页。
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
    (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// 无自定义请求头（大多数上游只需要默认头）。
pub const NO_HEADERS: [HttpHeader; 0] = [];

/// 词典抓词 HTTP 客户端：统一超时、UA、重定向策略与响应体上限。
///
/// `reqwest::Client` 内部共享连接池，进程内只构造一次（见 `DictState::new`）。
#[derive(Clone)]
pub struct DictHttp {
    client: Client,
}

impl DictHttp {
    /// 构造客户端：总超时 `timeout_ms`、连接超时 3 秒、最多跟随 3 次重定向。
    pub fn new(timeout_ms: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|e| {
                tracing::error!("构造词典 HTTP 客户端失败，回退默认客户端: {e}");
                Client::new()
            });
        Self { client }
    }

    /// GET 文本（UTF-8；上游均为 UTF-8 页面/接口）。
    pub async fn get_text(
        &self,
        url: &str,
        headers: &[HttpHeader],
    ) -> Result<String, DictProviderError> {
        let resp = self.send(self.client.get(url), headers).await?;
        read_text(resp).await
    }

    /// POST 表单文本（百度翻译等只接受表单提交的上游）。
    pub async fn post_form_text(
        &self,
        url: &str,
        form: FormBody<'_>,
        headers: &[HttpHeader],
    ) -> Result<String, DictProviderError> {
        let resp = self
            .send(self.client.post(url).form(&form), headers)
            .await?;
        read_text(resp).await
    }

    async fn send(
        &self,
        mut req: reqwest::RequestBuilder,
        headers: &[HttpHeader],
    ) -> Result<Response, DictProviderError> {
        for header in headers {
            req = req.header(header.name, header.value);
        }
        req.send().await.map_err(map_reqwest)
    }
}

/// 逐块读取响应体：累计超过 `MAX_BODY_BYTES` 立即返回 `TooLarge`。
async fn read_text(mut resp: Response) -> Result<String, DictProviderError> {
    let status = resp.status();
    if status == StatusCode::NOT_FOUND {
        return Err(DictProviderError::NotFound);
    }
    if !status.is_success() {
        return Err(DictProviderError::Http(format!("HTTP {status}")));
    }
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(map_reqwest)? {
        if buf.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(DictProviderError::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn map_reqwest(e: reqwest::Error) -> DictProviderError {
    if e.is_timeout() {
        DictProviderError::Timeout
    } else {
        DictProviderError::Http(e.to_string())
    }
}
