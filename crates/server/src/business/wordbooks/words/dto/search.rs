use serde::Deserialize;

/// 单词列表模式查询参数（搜索 + 排序 + 分页）。
#[derive(Debug, Deserialize)]
pub struct SearchWordsQuery {
    pub page: Option<String>,
    pub page_size: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
}
