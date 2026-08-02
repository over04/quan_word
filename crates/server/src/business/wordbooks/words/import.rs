//! 批量导入解析：csv（UTF-8/GBK）/ xlsx / xls / ods → 行数据 → 校验后的创建请求。
//!
//! 模板约定：第一行为表头 `单词,音标,词性,释义,例句`；每行一个单词；
//! 释义列多义项用 `；`/`;` 分隔，义项可自带词性前缀（如 `n. 放弃`）；
//! 词性列作为无前缀义项的统一词性；全空行跳过。

use std::io::Cursor;

use calamine::{Data, DataType};
use entity::definition::Definition;

use super::dto::create::CreateWordReq;
use super::error::WordError;
use super::service::WordService;

/// 导入数据行数上限（不含表头）。
pub const MAX_IMPORT_ROWS: usize = 5000;

/// 模板表头（csv 输出与各解析器跳过首行的依据）。
pub const HEADERS: [&str; 5] = ["单词", "音标", "词性", "释义", "例句"];

const VALID_POS: [&str; 13] = [
    "n.", "v.", "adj.", "adv.", "prep.", "conj.", "pron.", "num.", "art.", "interj.", "aux.",
    "abbr.", "phr.",
];

/// 模板行数据（表头跳过后的原始行，字段未 trim 语义保留原值）。
#[derive(Debug)]
pub struct RowData {
    pub spelling: String,
    pub phonetic: String,
    pub pos: String,
    pub meaning: String,
    pub example: String,
}

/// 按扩展名解析文件为行数据。结构性失败（损坏/编码不支持）返回单条 ImportFailed 明细。
pub fn parse_file(bytes: &[u8], ext: &str) -> Result<Vec<RowData>, WordError> {
    match ext.to_ascii_lowercase().as_str() {
        "csv" => parse_csv(bytes),
        "xlsx" => parse_spreadsheet::<calamine::Xlsx<_>>(bytes),
        "xls" => parse_spreadsheet::<calamine::Xls<_>>(bytes),
        "ods" => parse_spreadsheet::<calamine::Ods<_>>(bytes),
        _ => Err(WordError::UnsupportedFormat { ext: ext.into() }),
    }
}

/// 逐行转换为创建请求；任一行失败返回 `(文件行号, 错误消息)` 列表（行号从 2 起，含表头）。
pub fn to_create_reqs(rows: &[RowData]) -> Result<Vec<CreateWordReq>, Vec<(usize, String)>> {
    let mut reqs = Vec::new();
    let mut errors = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let line = i + 2; // 文件行号：第 1 行为表头
        let spelling = row.spelling.trim();
        if spelling.is_empty() {
            // 全空行跳过；只缺单词的报错
            if row.phonetic.trim().is_empty()
                && row.pos.trim().is_empty()
                && row.meaning.trim().is_empty()
                && row.example.trim().is_empty()
            {
                continue;
            }
            errors.push((line, "单词不能为空".into()));
            continue;
        }
        let definitions = parse_definitions(&row.meaning, &row.pos);
        if definitions.is_empty() {
            errors.push((line, "释义不能为空".into()));
            continue;
        }
        if let Err(e) = WordService::validate(spelling, &definitions) {
            errors.push((line, e.to_string()));
            continue;
        }
        reqs.push(CreateWordReq {
            spelling: spelling.to_string(),
            phonetic: non_empty(&row.phonetic),
            definitions,
            example: non_empty(&row.example),
        });
    }
    if errors.is_empty() {
        Ok(reqs)
    } else {
        Err(errors)
    }
}

/// 释义列拆分：按 `；`/`;` 分段，义项自带词性前缀则提取（大小写不敏感），否则用词性列。
fn parse_definitions(meaning: &str, pos_col: &str) -> Vec<Definition> {
    let pos_col = pos_col.trim();
    meaning
        .split(['；', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|item| match split_pos_prefix(item) {
            (Some(pos), rest) => Definition {
                pos,
                meaning: rest.to_string(),
            },
            (None, _) => Definition {
                pos: pos_col.to_string(),
                meaning: item.to_string(),
            },
        })
        .collect()
}

/// 义项词性前缀提取：`n. 放弃` / `N.放弃` → (Some("n."), "放弃")；无前缀返回 (None, 原义项)。
fn split_pos_prefix(item: &str) -> (Option<String>, &str) {
    let bytes = item.as_bytes();
    for pos in VALID_POS {
        if bytes.len() >= pos.len() && bytes[..pos.len()].eq_ignore_ascii_case(pos.as_bytes()) {
            // 前缀匹配成功说明前 pos.len() 字节均为 ASCII（与 pos 一致），字符边界安全
            return (Some(pos.to_string()), item[pos.len()..].trim());
        }
    }
    (None, item)
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_csv(bytes: &[u8]) -> Result<Vec<RowData>, WordError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = decode_text(bytes)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for (i, record) in rdr.records().enumerate() {
        if i == 0 {
            continue; // 表头
        }
        let record = record.map_err(|e| parse_error(format!("无法解析文件: {e}")))?;
        rows.push(RowData {
            spelling: record.get(0).unwrap_or("").to_string(),
            phonetic: record.get(1).unwrap_or("").to_string(),
            pos: record.get(2).unwrap_or("").to_string(),
            meaning: record.get(3).unwrap_or("").to_string(),
            example: record.get(4).unwrap_or("").to_string(),
        });
        if rows.len() > MAX_IMPORT_ROWS {
            return Err(WordError::TooManyRows {
                limit: MAX_IMPORT_ROWS,
            });
        }
    }
    Ok(rows)
}

/// 表格文件解析：UTF-8 优先，失败按 GBK 解码（中文 Excel/WPS 另存 CSV 常见 GBK）。
fn decode_text(bytes: &[u8]) -> Result<String, WordError> {
    std::str::from_utf8(bytes).map(str::to_string).or_else(|_| {
        let (decoded, _, had_errors) = encoding_rs::GBK.decode(bytes);
        if had_errors {
            Err(parse_error(
                "文件编码不支持，请使用 UTF-8 或 GBK 编码".into(),
            ))
        } else {
            Ok(decoded.into_owned())
        }
    })
}

/// xlsx / xls / ods 通用解析：取第一个工作表，跳过首行表头。
fn parse_spreadsheet<R>(bytes: &[u8]) -> Result<Vec<RowData>, WordError>
where
    R: calamine::Reader<Cursor<Vec<u8>>>,
{
    let mut wb = R::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| parse_error(format!("无法解析文件: {e:?}")))?;
    let name = wb
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| parse_error("文件中没有工作表".into()))?;
    let range = wb
        .worksheet_range(&name)
        .map_err(|e| parse_error(format!("无法解析文件: {e:?}")))?;
    let mut rows = Vec::new();
    for (i, row) in range.rows().enumerate() {
        if i == 0 {
            continue; // 表头
        }
        rows.push(RowData {
            spelling: cell(row, 0),
            phonetic: cell(row, 1),
            pos: cell(row, 2),
            meaning: cell(row, 3),
            example: cell(row, 4),
        });
        if rows.len() > MAX_IMPORT_ROWS {
            return Err(WordError::TooManyRows {
                limit: MAX_IMPORT_ROWS,
            });
        }
    }
    Ok(rows)
}

fn cell(row: &[Data], idx: usize) -> String {
    row.get(idx)
        .and_then(DataType::as_string)
        .unwrap_or_default()
}

fn parse_error(details: String) -> WordError {
    WordError::ImportFailed { count: 1, details }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_csv, parse_definitions, split_pos_prefix, to_create_reqs, RowData, HEADERS,
        MAX_IMPORT_ROWS,
    };
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn splits_pos_prefix_case_insensitive() {
        assert_eq!(split_pos_prefix("n. 放弃"), (Some("n.".into()), "放弃"));
        assert_eq!(split_pos_prefix("N.放弃"), (Some("n.".into()), "放弃"));
        assert_eq!(split_pos_prefix("放弃"), (None, "放弃"));
        assert_eq!(split_pos_prefix("n."), (Some("n.".into()), ""));
    }

    #[test]
    fn parses_definitions_with_mixed_separators() {
        let defs = parse_definitions("n. 放弃；v. 抛弃; 弃船", "");
        assert_eq!(defs.len(), 3);
        assert_eq!(defs[0].pos, "n.");
        assert_eq!(defs[0].meaning, "放弃");
        assert_eq!(defs[1].pos, "v.");
        assert_eq!(defs[1].meaning, "抛弃");
        assert_eq!(defs[2].pos, "");
        assert_eq!(defs[2].meaning, "弃船");
    }

    #[test]
    fn uses_pos_column_for_items_without_prefix() {
        let defs = parse_definitions("你好；喂", "intj.");
        assert_eq!(defs.len(), 2);
        assert!(defs.iter().all(|d| d.pos == "intj."));
    }

    #[test]
    fn parses_csv_skipping_header_and_bom() {
        let bytes = b"\xEF\xBB\xBF\xE5\x8D\x95\xE8\xAF\x8D,\xE9\x9F\xB3\xE6\xA0\x87,\xE8\xAF\x8D\xE6\x80\xA7,\xE9\x87\x8A\xE4\xB9\x89,\xE4\xBE\x8B\xE5\x8F\xA5\nhello,/,n.,\xe6\x94\xbe\xe5\xbc\x83,\n";
        let rows = parse_csv(bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spelling, "hello");
        assert_eq!(rows[0].meaning, "放弃");
    }

    #[test]
    fn converts_rows_to_reqs_with_line_numbers() {
        let rows = vec![
            RowData {
                spelling: "hello".into(),
                phonetic: "/h/".into(),
                pos: "".into(),
                meaning: "你好；喂".into(),
                example: "Hello!".into(),
            },
            RowData {
                spelling: "".into(),
                phonetic: "".into(),
                pos: "".into(),
                meaning: "".into(),
                example: "".into(),
            },
            RowData {
                spelling: "bad".into(),
                phonetic: "".into(),
                pos: "".into(),
                meaning: "".into(),
                example: "".into(),
            },
        ];
        let err = to_create_reqs(&rows).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].0, 4); // 第 4 行（空行在第 3 行被跳过）
        assert_eq!(err[0].1, "释义不能为空");
    }

    #[test]
    fn converts_valid_rows() {
        let rows = vec![RowData {
            spelling: "apple".into(),
            phonetic: "".into(),
            pos: "n.".into(),
            meaning: "苹果".into(),
            example: "".into(),
        }];
        let reqs = to_create_reqs(&rows).unwrap();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].spelling, "apple");
        assert_eq!(reqs[0].definitions[0].pos, "n.");
        assert_eq!(reqs[0].phonetic, None);
        assert_eq!(reqs[0].example, None);
    }

    #[test]
    fn rejects_unknown_extension() {
        let err = super::parse_file(b"", "pdf").unwrap_err();
        assert!(matches!(err, WordError::UnsupportedFormat { .. }));
    }

    #[test]
    fn rejects_over_limit_rows() {
        // 构造 5001 行 csv（含表头）
        let mut csv = String::from("单词,音标,词性,释义,例句\n");
        for _ in 0..=MAX_IMPORT_ROWS {
            csv.push_str("word,,,x,\n");
        }
        let err = parse_csv(csv.as_bytes()).unwrap_err();
        assert!(matches!(err, WordError::TooManyRows { .. }));
    }

    #[test]
    fn decodes_gbk_csv() {
        // 用 encoding_rs 生成 GBK 字节（中文 Excel/WPS 另存 CSV 常见编码）
        let (gbk_bytes, _, had_errors) =
            encoding_rs::GBK.encode("单词,音标,词性,释义,例句\nhi,,,你好,\n");
        assert!(!had_errors);
        let rows = parse_csv(&gbk_bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spelling, "hi");
        assert_eq!(rows[0].meaning, "你好");
        assert_eq!(HEADERS[0], "单词");
    }
}
