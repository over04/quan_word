use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::plugin_error::PluginError;

/// 词典源标识：小写字母开头，只含小写字母、数字、`_`、`-`，最长 32 字符。
///
/// 取代旧的封闭枚举：插件可以在运行时增加新 id，前端按字符串处理。
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, TS)]
#[serde(transparent)]
#[ts(export, export_to = "ProviderId.ts")]
pub struct ProviderId(String);

impl<'de> Deserialize<'de> for ProviderId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

impl ProviderId {
    pub fn new(raw: impl Into<String>) -> Result<Self, PluginError> {
        let raw = raw.into();
        if raw.is_empty() || raw.len() > 32 {
            return Err(PluginError::InvalidId(raw));
        }
        let mut chars = raw.chars();
        let Some(first) = chars.next() else {
            return Err(PluginError::InvalidId(raw));
        };
        if !first.is_ascii_lowercase() {
            return Err(PluginError::InvalidId(raw));
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        {
            return Err(PluginError::InvalidId(raw));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ProviderId {
    type Err = PluginError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::new(raw)
    }
}
#[cfg(test)]
mod tests {
    use super::ProviderId;

    #[test]
    fn accepts_plugin_ids() {
        for raw in ["youdao", "free_dict", "a-b", "a1"] {
            assert!(ProviderId::new(raw).is_ok(), "{raw}");
        }
    }

    #[test]
    fn rejects_invalid_plugin_ids() {
        for raw in ["", "Youdao", "1youdao", "a b", "a/b", "a.b"] {
            assert!(ProviderId::new(raw).is_err(), "{raw}");
        }
        assert!(ProviderId::new("a".repeat(33)).is_err());
    }

    #[test]
    fn deserializes_with_validation() {
        assert!(serde_json::from_str::<ProviderId>("\"youdao\"").is_ok());
        assert!(serde_json::from_str::<ProviderId>("\"Youdao\"").is_err());
    }
}
