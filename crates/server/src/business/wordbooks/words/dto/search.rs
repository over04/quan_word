use serde::Deserialize;

/// 单词列表模式查询参数（搜索 + 排序 + 分页 + 标签筛选）。
#[derive(Debug, Deserialize)]
pub struct SearchWordsQuery {
    pub page: Option<String>,
    pub page_size: Option<String>,
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    /// 标签筛选（组数组 JSON）：[{"mode":"and"|"or","ids":[1,2]},...]；组间取交集，组内按 mode
    pub tag_groups: Option<String>,
}
