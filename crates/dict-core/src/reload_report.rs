use crate::plugin_info::PluginInfo;
use crate::plugin_load_report::PluginLoadReport;

/// 一次插件重载的汇总。
#[derive(Debug, Clone)]
pub struct ReloadReport {
    pub loaded: usize,
    pub rejected: usize,
    pub plugins: Vec<PluginInfo>,
    pub reports: Vec<PluginLoadReport>,
}
