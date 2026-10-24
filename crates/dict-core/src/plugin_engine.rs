use serde::Deserialize;

/// 插件执行引擎。
///
/// 当前只实现 `recipe`；`wasm` 是预留逃生舱，装载期显式拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginEngine {
    Recipe,
    Wasm,
}
