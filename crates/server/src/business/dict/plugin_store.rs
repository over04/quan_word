use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use dict_core::plugin_load_report::PluginLoadReport;
use dict_core::plugin_source::PluginSource;
use dict_core::plugin_status::PluginStatus;
use dict_core::provider_id::ProviderId;

use super::plugin_sources::PluginSources;
use crate::config::dict::DictConfig;

/// 运行时插件目录：只读取 `plugins_dir` 下的 `*.json`，不内置任何插件。
pub struct PluginStore {
    dir: PathBuf,
}

impl PluginStore {
    pub fn new(config: &DictConfig) -> Self {
        Self {
            dir: PathBuf::from(&config.plugins_dir),
        }
    }

    pub fn collect(&self) -> PluginSources {
        let mut sources = Vec::new();
        let mut failures = Vec::new();
        let files = match self.scan_external() {
            Ok(files) => files,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return PluginSources { sources, failures };
            }
            Err(error) => {
                failures.push(rejected(
                    self.dir.display().to_string(),
                    format!("插件目录读取失败: {error}"),
                ));
                return PluginSources { sources, failures };
            }
        };
        for path in files {
            let file = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string());
            match fs::read(&path) {
                Ok(bytes) => sources.push(PluginSource::new(file, bytes)),
                Err(error) => failures.push(rejected(file, format!("插件读取失败: {error}"))),
            }
        }
        PluginSources { sources, failures }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 当前插件文件（已排序，跳过隐藏状态文件）。
    pub fn external_paths(&self) -> Vec<PathBuf> {
        self.scan_external().unwrap_or_default()
    }

    /// 原子写入插件包，文件名统一为 `<id>.json`。
    pub fn write_external(&self, id: &ProviderId, bytes: &[u8]) -> std::io::Result<PathBuf> {
        fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!("{id}.json"));
        let tmp = self.dir.join(format!(".{id}.json.tmp"));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &path)?;
        Ok(path)
    }

    /// 启停状态文件：放在插件目录内，扫描时按隐藏文件跳过。
    pub fn prefs_path(&self) -> PathBuf {
        super::plugin_prefs::state_path(&self.dir)
    }

    /// 目录指纹：文件名 + 长度 + 修改时间。轮询只在变化时重载，避免无谓清缓存。
    pub fn signature(&self) -> String {
        let mut parts = Vec::new();
        match fs::read_dir(&self.dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !is_plugin_file(&path) {
                        continue;
                    }
                    let metadata = entry.metadata().ok();
                    let len = metadata.as_ref().map_or(0, |value| value.len());
                    let modified = metadata
                        .and_then(|value| value.modified().ok())
                        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                        .map_or(0, |value| value.as_nanos());
                    let name = path
                        .file_name()
                        .map(|value| value.to_string_lossy().to_string())
                        .unwrap_or_default();
                    parts.push(format!("{name}:{len}:{modified}"));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => parts.push(format!("error:{:?}", error.kind())),
        }
        parts.sort();
        parts.join("|")
    }

    fn scan_external(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut files: Vec<PathBuf> = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| is_plugin_file(path))
            .collect();
        files.sort();
        Ok(files)
    }
}

fn is_plugin_file(path: &Path) -> bool {
    let visible = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.starts_with('.'));
    visible
        && path
            .extension()
            .is_some_and(|extension| extension == "json")
}

fn rejected(file: String, error: String) -> PluginLoadReport {
    PluginLoadReport {
        file,
        id: None,
        status: PluginStatus::Rejected,
        error: Some(error),
    }
}
