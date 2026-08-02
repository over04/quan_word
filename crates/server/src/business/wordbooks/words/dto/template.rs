use serde::Deserialize;

/// 模板下载查询参数：format = csv | xlsx，缺省 csv。
#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub format: Option<String>,
}
