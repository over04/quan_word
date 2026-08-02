/// HTTP 监听配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// 访问密钥：设置后所有 /api 请求必须携带 Authorization: Bearer <密钥>
    /// （或 X-Api-Key 头）；未设置（None/空）则不启用鉴权。
    #[serde(default)]
    pub auth_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 3000,
            auth_key: None,
        }
    }
}
