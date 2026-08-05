use serde::Deserialize;

use super::tag_match::TagMatch;

/// 一个标签筛选组：组内标签按 `mode` 匹配（And=全部命中 / Or=任一命中）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct TagGroup {
    #[serde(
        deserialize_with = "crate::business::wordbooks::words::tag_match::deserialize_tag_match"
    )]
    pub mode: TagMatch,
    pub ids: Vec<i32>,
}

/// `tag_groups` 查询参数的 JSON 结构：组数组 + 组间连接词数组。
///
/// `links[i]` 为组 `i` 与组 `i+1` 之间的连接词（And=且 / Or=或）；
/// 长度必须等于 `groups.len() - 1`（单组时为空数组），由 `WordService::parse_tag_groups` 校验。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct TagGroupsParam {
    pub groups: Vec<TagGroup>,
    #[serde(deserialize_with = "crate::business::wordbooks::words::tag_match::deserialize_links")]
    pub links: Vec<TagMatch>,
}
