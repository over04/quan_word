use serde::Deserialize;

/// 单词列表模式查询参数（搜索 + 排序 + 分页 + 标签筛选）。
#[derive(Debug, Deserialize)]
pub struct SearchWordsQuery {
    pub page: Option<String>,
    pub page_size: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    /// 标签筛选（逗号分隔的标签 id，多选交集）
    pub tag: Option<String>,
    /// 标签匹配模式：and=交集（默认）/ or=并集
    pub tag_match: Option<String>,
}
