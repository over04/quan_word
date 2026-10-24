use dict_core::plugin_load_report::PluginLoadReport;
use dict_core::plugin_source::PluginSource;

/// 一次扫描得到的插件字节 + 读取阶段失败报告（解析失败由引擎报告）。
pub struct PluginSources {
    pub sources: Vec<PluginSource>,
    pub failures: Vec<PluginLoadReport>,
}
