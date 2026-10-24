use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use dict_core::provider_id::ProviderId;

/// 插件启停偏好：运行时覆盖配置默认值，落盘到插件目录下的隐藏状态文件。
///
/// 语义：
/// - `disabled` 永远优先，记录用户明确关闭的插件；
/// - `extra_enabled` 记录配置 `dict.providers` 之外、被用户手动打开的插件；
/// - 配置 `providers: []` 时，未出现在 `disabled` 里的插件默认全部启用。
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PluginPrefs {
    /// 内存态不落盘；生产态指向 `plugins_dir/.state.json`。
    #[serde(skip)]
    path: Option<PathBuf>,
    #[serde(default)]
    disabled: BTreeSet<ProviderId>,
    #[serde(default)]
    extra_enabled: BTreeSet<ProviderId>,
}

impl PluginPrefs {
    /// 读取状态文件；文件不存在或损坏时回退空偏好，不阻塞启动。
    pub fn load(path: PathBuf) -> Self {
        match fs::read_to_string(&path) {
            Ok(raw) => match serde_json::from_str::<Self>(&raw) {
                Ok(mut prefs) => {
                    prefs.path = Some(path);
                    prefs
                }
                Err(error) => {
                    tracing::warn!(path = %path.display(), %error, "插件启停状态解析失败，使用默认值");
                    Self {
                        path: Some(path),
                        ..Self::default()
                    }
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self {
                path: Some(path),
                ..Self::default()
            },
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "插件启停状态读取失败，使用默认值");
                Self {
                    path: Some(path),
                    ..Self::default()
                }
            }
        }
    }

    /// 测试/嵌入场景：不读写磁盘。
    pub fn memory() -> Self {
        Self::default()
    }

    pub fn is_disabled(&self, id: &ProviderId) -> bool {
        self.disabled.contains(id)
    }

    pub fn is_extra_enabled(&self, id: &ProviderId) -> bool {
        self.extra_enabled.contains(id)
    }

    /// 更新单个插件的启停偏好；`outside_config` 表示该 id 不在 `dict.providers` 里。
    pub fn set_enabled(&mut self, id: &ProviderId, enabled: bool, outside_config: bool) {
        if enabled {
            self.disabled.remove(id);
            if outside_config {
                self.extra_enabled.insert(id.clone());
            } else {
                self.extra_enabled.remove(id);
            }
        } else {
            self.disabled.insert(id.clone());
            self.extra_enabled.remove(id);
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        let Some(parent) = path.parent() else {
            return;
        };
        if let Err(error) = fs::create_dir_all(parent) {
            tracing::warn!(path = %path.display(), %error, "创建插件状态目录失败");
            return;
        }
        let body = match serde_json::to_vec_pretty(self) {
            Ok(body) => body,
            Err(error) => {
                tracing::warn!(%error, "序列化插件启停状态失败");
                return;
            }
        };
        let tmp = path.with_extension("json.tmp");
        if let Err(error) = fs::write(&tmp, body).and_then(|()| fs::rename(&tmp, path)) {
            tracing::warn!(path = %path.display(), %error, "写入插件启停状态失败");
        }
    }
}

/// 状态文件放在插件目录内，避免额外配置；隐藏文件名不会被插件扫描器当作插件。
pub fn state_path(plugins_dir: &Path) -> PathBuf {
    plugins_dir.join(".state.json")
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{state_path, PluginPrefs};
    use dict_core::provider_id::ProviderId;

    fn id(raw: &str) -> ProviderId {
        ProviderId::new(raw).unwrap()
    }

    fn temp_dir() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("quan-word-prefs-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn persists_disabled_and_extra_enabled() {
        let dir = temp_dir();
        let path = state_path(&dir);
        let mut prefs = PluginPrefs::load(path.clone());
        prefs.set_enabled(&id("youdao"), false, false);
        prefs.set_enabled(&id("custom"), true, true);
        drop(prefs);

        let loaded = PluginPrefs::load(path);
        assert!(loaded.is_disabled(&id("youdao")));
        assert!(loaded.is_extra_enabled(&id("custom")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn memory_prefs_do_not_touch_disk() {
        let mut prefs = PluginPrefs::memory();
        prefs.set_enabled(&id("youdao"), false, false);
        assert!(prefs.is_disabled(&id("youdao")));
    }
}
