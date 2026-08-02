use serde::Deserialize;

/// 单词列表（纸质书浏览）查询参数。
#[derive(Debug, Deserialize)]
pub struct ListWordsQuery {
    pub page: Option<String>,
    pub page_size: Option<String>,
    pub order: Option<String>,
    pub seed: Option<String>,
}
