use serde::Deserialize;
use ts_rs::TS;

/// 替换单词标签集请求体（全量替换，与创建/更新同语义）。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "UpdateWordTagsReq.ts")]
pub struct UpdateWordTagsReq {
    /// 标签 id 数组（须属于该书），缺省为空
    #[serde(default)]
    pub tags: Vec<i32>,
}
