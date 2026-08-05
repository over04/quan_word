use serde::Deserialize;

/// 单词列表（纸质书浏览）查询参数。
#[derive(Debug, Deserialize)]
pub struct ListWordsQuery {
    pub page: Option<String>,
    pub page_size: Option<String>,
    pub order: Option<String>,
    pub seed: Option<String>,
    /// 标签筛选（组数组 JSON）：[{"mode":"and"|"or","ids":[1,2]},...]；组间取交集，组内按 mode
    pub tag_groups: Option<String>,
}
