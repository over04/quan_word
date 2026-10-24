use crate::plugin_info::PluginInfo;
use crate::plugin_limits::PluginLimits;
use crate::recipe::Recipe;

/// 注册表里的一条：公开信息 + 配方 + 插件级上限 + 来源文件。
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    pub info: PluginInfo,
    pub recipe: Recipe,
    pub limits: PluginLimits,
    pub file: String,
}
