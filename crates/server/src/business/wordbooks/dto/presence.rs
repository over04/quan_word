use serde::Deserialize;

/// 跨单词书查询某拼写是否已存在。
#[derive(Debug, Deserialize)]
pub struct WordPresenceQuery {
    pub spelling: Option<String>,
}
