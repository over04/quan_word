use serde::Serialize;
use ts_rs::TS;

/// 批量导入响应体：成功导入的单词数（导入为原子操作，任一行失败则整体失败并返回 400）。
#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "ImportResp.ts")]
pub struct ImportResp {
    #[ts(type = "number")]
    pub imported: u64,
}
