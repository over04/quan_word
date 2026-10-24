//! 仓库维护命令：校验示例插件包。
//!
//! 用法：
//! - `cargo run -p xtask -- validate-plugins`：校验 `examples/plugins` 下的示例
//! - `cargo run -p xtask -- validate-plugins <目录>`：校验任意插件目录

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dict_core::engine_limits::EngineLimits;
use dict_core::plugin_registry::PluginRegistry;
use dict_core::plugin_source::PluginSource;

const EXAMPLES: &str = "examples/plugins";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first().map(String::as_str) else {
        usage();
        return ExitCode::FAILURE;
    };
    match command {
        "validate-plugins" => validate_plugins(args.get(1).map(String::as_str)),
        "-h" | "--help" | "help" => {
            usage();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("未知命令: {other}");
            usage();
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!("用法:");
    eprintln!("  cargo run -p xtask -- validate-plugins [插件目录，默认 {EXAMPLES}]");
}

fn validate_plugins(dir: Option<&str>) -> ExitCode {
    let dir = dir.unwrap_or(EXAMPLES);
    let sources = match read_directory(Path::new(dir)) {
        Ok(sources) => sources,
        Err(error) => {
            eprintln!("读取插件目录失败: {error}");
            return ExitCode::FAILURE;
        }
    };
    if sources.is_empty() {
        eprintln!("没有找到任何插件包");
        return ExitCode::FAILURE;
    }
    let registry = PluginRegistry::build(sources, &EngineLimits::default());
    let mut failed = 0;
    for report in registry.reports() {
        match report.status {
            dict_core::plugin_status::PluginStatus::Loaded => println!(
                "ok   {:<28} id={}",
                report.file,
                report.id.as_ref().map_or("-", |id| id.as_str())
            ),
            dict_core::plugin_status::PluginStatus::Rejected => {
                failed += 1;
                eprintln!(
                    "fail {:<28} {}",
                    report.file,
                    report.error.as_deref().unwrap_or("未知错误")
                );
            }
        }
    }
    if failed > 0 {
        eprintln!("{failed}/{} 个插件包校验失败", registry.reports().len());
        return ExitCode::FAILURE;
    }
    println!("{} 个插件包校验通过", registry.len());
    ExitCode::SUCCESS
}

/// 读取目录下全部可见 `*.json`（按文件名排序）。
fn read_directory(dir: &Path) -> std::io::Result<Vec<PluginSource>> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| !name.starts_with('.'))
                && path
                    .extension()
                    .is_some_and(|extension| extension == "json")
        })
        .collect();
    files.sort();
    let mut sources = Vec::with_capacity(files.len());
    for path in files {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        sources.push(PluginSource::new(file, fs::read(&path)?));
    }
    Ok(sources)
}
