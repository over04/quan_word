use lexicon::definition::Definition;
use serde::Serialize;
use ts_rs::TS;

use crate::example::DictExample;
use crate::provider_id::ProviderId;

/// 单个词典源返回的词条（聚合响应的条目单元）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "DictEntry.ts")]
pub struct DictEntry {
    /// 来源词典源
    pub provider: ProviderId,
    /// 来源中文名
    pub label: String,
    /// 查询到的单词（上游规范化后的拼写）
    pub word: String,
    /// 音标（形如 /əˈbændən/）；None = 该源未提供
    pub phonetic: Option<String>,
    /// 释义（词性 + 释义，复用单词本义项模型）
    pub definitions: Vec<Definition>,
    /// 例句
    pub examples: Vec<DictExample>,
    /// 发音音频地址
    pub audio_url: Option<String>,
    /// 上游词条页地址
    pub source_url: Option<String>,
}
