use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 批量导入响应体。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "ImportResp.ts")]
pub struct ImportResp {
    /// 新增单词数
    #[ts(type = "number")]
    pub imported: u64,
    /// 覆盖更新的重复单词数
    #[ts(type = "number")]
    pub updated: u64,
    /// 校验失败被跳过的行（含修正后仍失败）
    #[ts(type = "number")]
    pub skipped_errors: u64,
    /// 重复但未选择更新的行
    #[ts(type = "number")]
    pub skipped_duplicates: u64,
    /// 自动新建的标签数
    #[ts(type = "number")]
    pub created_tags: u64,
}

/// 导入预览响应体（解析结果，不落库）。rows 为**当前页**行视图（分页由后端会话内计算），
/// 统计为全量；翻页/编辑/筛选走 `POST /import/rows`。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "ImportPreviewResp.ts")]
pub struct ImportPreviewResp {
    /// 会话凭据（翻页/编辑/导入执行时提交）
    pub token: String,
    /// 数据行总数（不含全空行）
    #[ts(type = "number")]
    pub total_rows: u64,
    /// 无错误的行
    #[ts(type = "number")]
    pub valid_rows: u64,
    /// 有错误的行
    #[ts(type = "number")]
    pub invalid_rows: u64,
    /// 重复组总数
    #[ts(type = "number")]
    pub duplicate_total: u64,
    /// 全部重复组的组首行号（跨页全量；前端「全部更新/全部跳过」用）
    #[ts(type = "Array<number>")]
    pub duplicate_groups: Vec<u64>,
    /// 当前页行视图（行号升序）
    pub rows: Vec<ImportRowView>,
    /// 文件中出现、该书不存在的标签名（trim 后，出现顺序去重）
    pub new_tags: Vec<String>,
    /// 文件中出现且该书已存在的标签数
    #[ts(type = "number")]
    pub existing_tags: u64,
    /// 当前页码（1 起）
    #[ts(type = "number")]
    pub page: u64,
    /// 页大小
    #[ts(type = "number")]
    pub page_size: u64,
    /// 总页数（按当前筛选）
    #[ts(type = "number")]
    pub total_pages: u64,
}

/// 预览行视图：原始列值 + 行状态（前端表格可编辑；错误/重复标记只读，由后端解析时判定）。
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "ImportRowView.ts")]
pub struct ImportRowView {
    /// 文件行号（第 1 行为表头，数据从第 2 行起）
    #[ts(type = "number")]
    pub row: u64,
    pub spelling: String,
    pub phonetic: String,
    pub pos: String,
    pub meaning: String,
    pub example: String,
    pub tags: String,
    /// 校验错误消息；None = 通过（仍可编辑）
    pub error: Option<String>,
    /// 与书内已有词重复（trim + 小写比较）
    pub is_duplicate: bool,
    /// 所属单词组的首行行号（同词多义项行共享；前端按组显示「更新」勾选）
    #[ts(type = "number")]
    pub group: u64,
}

/// 模板行数据（预览回传 / 错误修正共用；row 为文件行号）。
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ImportRowData.ts")]
pub struct ImportRowData {
    #[ts(type = "number")]
    pub row: u64,
    pub spelling: String,
    pub phonetic: String,
    pub pos: String,
    pub meaning: String,
    pub example: String,
    pub tags: String,
}

/// 导入执行请求体：token 会话 + 重复行处理策略（行修正已实时提交到会话）。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "ImportExecReq.ts")]
pub struct ImportExecReq {
    pub token: String,
    /// 标记「更新」的重复组行号（组内任一行命中即整组更新）；其余重复组跳过
    #[serde(default)]
    #[ts(type = "Array<number>")]
    pub update_rows: Vec<u64>,
}

/// 导入行分页/编辑请求体：翻页、筛选、提交行级修正（会话内重算后返回当前页）。
#[derive(Debug, Deserialize, TS)]
#[ts(export, export_to = "ImportRowsReq.ts")]
pub struct ImportRowsReq {
    pub token: String,
    /// 页码（1 起）
    #[ts(type = "number")]
    pub page: u64,
    /// 页大小（1..=100）
    #[ts(type = "number")]
    pub page_size: u64,
    /// 筛选：all | error | duplicate
    #[serde(default = "default_filter")]
    pub filter: String,
    /// 行级修正（编辑草稿，覆盖对应行号后重新校验；空 = 纯翻页/筛选）
    #[serde(default)]
    pub updates: Vec<ImportRowData>,
}

fn default_filter() -> String {
    "all".to_string()
}

/// 行分页响应：与预览一致但无 token（会话凭据由请求方持有）。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "ImportRowsResp.ts")]
pub struct ImportRowsResp {
    #[ts(type = "number")]
    pub total_rows: u64,
    #[ts(type = "number")]
    pub valid_rows: u64,
    #[ts(type = "number")]
    pub invalid_rows: u64,
    #[ts(type = "number")]
    pub duplicate_total: u64,
    #[ts(type = "Array<number>")]
    pub duplicate_groups: Vec<u64>,
    pub rows: Vec<ImportRowView>,
    pub new_tags: Vec<String>,
    #[ts(type = "number")]
    pub existing_tags: u64,
    #[ts(type = "number")]
    pub page: u64,
    #[ts(type = "number")]
    pub page_size: u64,
    #[ts(type = "number")]
    pub total_pages: u64,
}

/// 预览接口的分页查询参数（缺省 page=1、page_size=25）。
#[derive(Debug, Deserialize)]
pub struct PreviewPageQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
