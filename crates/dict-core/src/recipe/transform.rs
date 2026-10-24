use serde::Deserialize;

use super::unit_transform::UnitTransform;

/// 单步变换：字符串形式是单位变换，单键对象是带参变换。
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Transform {
    Unit(UnitTransform),
    Join { join: String },
    Split { split: Vec<String> },
    Take { take: usize },
    Dedup { dedup: String },
    StripChars { strip_chars: Vec<String> },
    Between { between: [String; 2] },
    Before { before: String },
    After { after: String },
    AfterLast { after_last: String },
    TrimChars { trim_chars: String },
    CapChars { cap_chars: usize },
    Default { default: String },
}
