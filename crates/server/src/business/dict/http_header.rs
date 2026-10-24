/// HTTP 请求头（静态常量声明，避免匿名元组在调用链上传递）。
#[derive(Debug, Clone, Copy)]
pub struct HttpHeader {
    pub name: &'static str,
    pub value: &'static str,
}

impl HttpHeader {
    pub const fn new(name: &'static str, value: &'static str) -> Self {
        Self { name, value }
    }
}
