use serde::Deserialize;

/// 抓词查询参数。
///
/// 仓库约定 Query 不做 rejection 解析：字段全部为可选字符串，由 router 解析为
/// 枚举/布尔值后再交给 service（非法值返回中文 400）。
#[derive(Debug, Deserialize)]
pub struct LookupQuery {
    /// 要查询的单词
    pub word: Option<String>,
    /// 逗号分隔的词典源 id；缺省 = 配置里启用的全部源
    pub providers: Option<String>,
    /// 1/true/yes/on 时跳过缓存强制刷新
    pub refresh: Option<String>,
}
