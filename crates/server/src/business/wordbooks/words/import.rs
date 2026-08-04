//! 批量导入解析：csv（UTF-8/GBK）/ xlsx / xls / ods → 行数据 → 行级校验 → 按拼写分组。
//!
//! 模板约定：第一行为表头 `单词,音标,词性,释义,例句,标签`；**每行一个义项**
//! （一个词性对应一个释义，与列表/表单一致）；同一单词的多个义项写多行
//! （单词列重复填写），导入时按拼写合并为一个单词、按行序拼接义项；
//! 音标/例句取组内首个非空值，标签取组内并集；全空行跳过。
//! 行数上限由配置 `import.max_rows` 决定。

use std::io::Cursor;

use calamine::{Data, DataType};
use entity::definition::Definition;

use super::dto::import::ImportRowData;
use super::error::WordError;
use super::file_type::ImportFileType;
use super::service::WordService;

/// 模板表头（csv 输出与各解析器跳过首行的依据）。
pub const HEADERS: [&str; 6] = ["单词", "音标", "词性", "释义", "例句", "标签"];

/// 模板行数据（表头跳过后的原始行，字段未 trim 语义保留原值；row_no 为文件行号，从 2 起）。
#[derive(Debug)]
pub struct RowData {
    pub row_no: usize,
    pub spelling: String,
    pub phonetic: String,
    pub pos: String,
    pub meaning: String,
    pub example: String,
    pub tags: String,
}

impl RowData {
    /// 六列全部 trim 后为空（与 prepare_rows 跳过规则一致）。
    pub(crate) fn is_blank(&self) -> bool {
        self.spelling.trim().is_empty()
            && self.phonetic.trim().is_empty()
            && self.pos.trim().is_empty()
            && self.meaning.trim().is_empty()
            && self.example.trim().is_empty()
            && self.tags.trim().is_empty()
    }

    /// 转 DTO（行号取自身）。
    pub(crate) fn to_dto(&self) -> ImportRowData {
        ImportRowData {
            row: self.row_no as u64,
            spelling: self.spelling.clone(),
            phonetic: self.phonetic.clone(),
            pos: self.pos.clone(),
            meaning: self.meaning.clone(),
            example: self.example.clone(),
            tags: self.tags.clone(),
        }
    }

    /// 从 DTO 还原（行号取 d.row）。
    pub(crate) fn from_dto(d: &ImportRowData) -> Self {
        Self {
            row_no: d.row as usize,
            spelling: d.spelling.clone(),
            phonetic: d.phonetic.clone(),
            pos: d.pos.clone(),
            meaning: d.meaning.clone(),
            example: d.example.clone(),
            tags: d.tags.clone(),
        }
    }
}

/// 按文件类型解析为行数据。结构性失败（损坏/编码不支持）返回单条 ImportFailed 明细。
/// 数据行数超过 `max_rows` 时返回 TooManyRows。
pub fn parse_file(
    bytes: &[u8],
    file_type: ImportFileType,
    max_rows: usize,
) -> Result<Vec<RowData>, WordError> {
    match file_type {
        ImportFileType::Csv => parse_csv(bytes, max_rows),
        ImportFileType::Xlsx => parse_spreadsheet::<calamine::Xlsx<_>>(bytes, max_rows),
        ImportFileType::Xls => parse_spreadsheet::<calamine::Xls<_>>(bytes, max_rows),
        ImportFileType::Ods => parse_spreadsheet::<calamine::Ods<_>>(bytes, max_rows),
    }
}

/// 已通过行级校验的导入行（每行 = 一个词性/释义义项）。
#[derive(Debug)]
pub struct PreparedRow {
    pub row_no: usize,    // 文件行号（从 2 起）
    pub spelling: String, // 已 trim
    pub phonetic: Option<String>,
    pub pos: String,     // 该义项词性（已 trim；允许空）
    pub meaning: String, // 该义项释义（已 trim）
    pub example: Option<String>,
    pub tag_names: Vec<String>, // parse_tags 结果（已 trim/去重）
}

/// 按拼写合并后的单词（组内全部义项 + 组级字段）。
#[derive(Debug)]
pub struct WordGroup {
    pub row_nos: Vec<usize>,          // 组内行号（升序）
    pub spelling: String,             // 已 trim
    pub phonetic: Option<String>,     // 组内首个非空
    pub example: Option<String>,      // 组内首个非空
    pub tag_names: Vec<String>,       // 组内并集（去重，出现顺序）
    pub definitions: Vec<Definition>, // 按行序 (pos, meaning)
}

/// 逐行校验并分类：返回（有效行，错误明细）。容错：错误行跳过，有效行照常产出。
///
/// 规则：全空行跳过不计数；单词空 → "单词不能为空"；释义空 → "释义不能为空"；
/// 词性白名单/其余校验走 `WordService::validate`；标签解析失败 → parse_tags 的消息。
/// 行号从 2 起（`rows[i]` ↔ 文件行 `i+2`，parse 阶段不跳空行，索引连续）。
pub fn prepare_rows(rows: &[RowData]) -> (Vec<PreparedRow>, Vec<(usize, String)>) {
    let mut prepared = Vec::new();
    let mut errors = Vec::new();
    for row in rows {
        let line = row.row_no; // 文件行号（第 1 行为表头）
        let spelling = row.spelling.trim();
        if spelling.is_empty() {
            if row.is_blank() {
                continue;
            }
            errors.push((line, "单词不能为空".into()));
            continue;
        }
        let meaning = row.meaning.trim();
        if meaning.is_empty() {
            errors.push((line, "释义不能为空".into()));
            continue;
        }
        let pos = row.pos.trim();
        if let Err(e) = WordService::validate(
            spelling,
            &[Definition {
                pos: pos.to_string(),
                meaning: meaning.to_string(),
            }],
        ) {
            errors.push((line, e.to_string()));
            continue;
        }
        let tag_names = match parse_tags(&row.tags) {
            Ok(names) => names,
            Err(msg) => {
                errors.push((line, msg));
                continue;
            }
        };
        prepared.push(PreparedRow {
            row_no: line,
            spelling: spelling.to_string(),
            phonetic: non_empty(&row.phonetic),
            pos: pos.to_string(),
            meaning: meaning.to_string(),
            example: non_empty(&row.example),
            tag_names,
        });
    }
    (prepared, errors)
}

/// 把行义项按拼写（trim + 小写）分组为单词：组序按首次出现顺序，
/// 组内 definitions 按行序拼接；音标/例句取首个非空，标签取并集。
pub fn group_rows(rows: &[PreparedRow]) -> Vec<WordGroup> {
    let mut groups: Vec<WordGroup> = Vec::new();
    for row in rows {
        let key = row.spelling.to_lowercase();
        let group = match groups.iter_mut().find(|g| g.spelling.to_lowercase() == key) {
            Some(g) => g,
            None => {
                groups.push(WordGroup {
                    row_nos: Vec::new(),
                    spelling: row.spelling.clone(),
                    phonetic: None,
                    example: None,
                    tag_names: Vec::new(),
                    definitions: Vec::new(),
                });
                groups.last_mut().expect("刚 push")
            }
        };
        group.row_nos.push(row.row_no);
        if group.phonetic.is_none() {
            group.phonetic = row.phonetic.clone();
        }
        if group.example.is_none() {
            group.example = row.example.clone();
        }
        for name in &row.tag_names {
            if !group.tag_names.contains(name) {
                group.tag_names.push(name.clone());
            }
        }
        group.definitions.push(Definition {
            pos: row.pos.clone(),
            meaning: row.meaning.clone(),
        });
    }
    groups
}

/// 标签列解析：按 `；`/`;` 分段、trim、忽略空段、行内去重；
/// 任一标签超过 20 字符（与 TagService::TAG_NAME_MAX 一致）返回 Err(消息)。
fn parse_tags(raw: &str) -> Result<Vec<String>, String> {
    const MAX_TAG_LEN: usize = 20;
    let mut seen: Vec<String> = Vec::new();
    for item in raw.split(['；', ';']) {
        let name = item.trim();
        if name.is_empty() || seen.iter().any(|s| s == name) {
            continue;
        }
        if name.chars().count() > MAX_TAG_LEN {
            return Err(format!("标签「{name}」不能超过 20 个字符"));
        }
        seen.push(name.to_string());
    }
    Ok(seen)
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn parse_csv(bytes: &[u8], max_rows: usize) -> Result<Vec<RowData>, WordError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = decode_text(bytes)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    let mut record = csv::StringRecord::new();
    let mut idx = 0usize;
    while rdr
        .read_record(&mut record)
        .map_err(|e| parse_error(format!("无法解析文件: {e}")))?
    {
        if idx == 0 {
            idx += 1;
            continue; // 表头
        }
        idx += 1;
        // position().line() 为物理行号（1 起，含空行；csv 库会丢弃完全空行，不能依赖记录下标）。
        // 读取记录后 position 指向该记录的下一行，故减 1 得当前记录行号。
        let row_no = rdr.position().line() as usize - 1;
        rows.push(RowData {
            row_no,
            spelling: record.get(0).unwrap_or("").to_string(),
            phonetic: record.get(1).unwrap_or("").to_string(),
            pos: record.get(2).unwrap_or("").to_string(),
            meaning: record.get(3).unwrap_or("").to_string(),
            example: record.get(4).unwrap_or("").to_string(),
            tags: record.get(5).unwrap_or("").to_string(),
        });
        if rows.len() > max_rows {
            return Err(WordError::TooManyRows { limit: max_rows });
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
fn parse_spreadsheet<R>(bytes: &[u8], max_rows: usize) -> Result<Vec<RowData>, WordError>
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
            row_no: i + 1, // 表头占第 0 行，数据行从文件第 2 行起
            spelling: cell(row, 0),
            phonetic: cell(row, 1),
            pos: cell(row, 2),
            meaning: cell(row, 3),
            example: cell(row, 4),
            tags: cell(row, 5),
        });
        if rows.len() > max_rows {
            return Err(WordError::TooManyRows { limit: max_rows });
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
    use super::{group_rows, parse_csv, parse_tags, prepare_rows, PreparedRow, RowData, HEADERS};
    use crate::business::wordbooks::words::error::WordError;

    #[test]
    fn parses_tags_with_separators_trim_and_dedup() {
        assert_eq!(
            parse_tags("水果；高频"),
            Ok(vec!["水果".into(), "高频".into()])
        );
        assert_eq!(
            parse_tags("水果; 高频 ;水果"),
            Ok(vec!["水果".into(), "高频".into()])
        );
        assert_eq!(parse_tags("  "), Ok(vec![]));
        assert_eq!(parse_tags("；；"), Ok(vec![]));
        assert_eq!(parse_tags(""), Ok(vec![]));
    }

    #[test]
    fn rejects_overlong_tag() {
        let long = "超".repeat(21);
        let err = parse_tags(&format!("高频；{long}")).unwrap_err();
        assert!(err.contains("不能超过 20 个字符"));
    }

    #[test]
    fn parses_csv_skipping_header_and_bom() {
        let bytes = b"\xEF\xBB\xBF\xE5\x8D\x95\xE8\xAF\x8D,\xE9\x9F\xB3\xE6\xA0\x87,\xE8\xAF\x8D\xE6\x80\xA7,\xE9\x87\x8A\xE4\xB9\x89,\xE4\xBE\x8B\xE5\x8F\xA5,\xE6\xA0\x87\xE7\xAD\xBE\nhello,/,n.,\xe6\x94\xbe\xe5\xbc\x83,,\xe9\xab\x98\xe9\xa2\x91\n";
        let rows = parse_csv(bytes, 5000).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spelling, "hello");
        assert_eq!(rows[0].meaning, "放弃");
        assert_eq!(rows[0].tags, "高频");
    }

    #[test]
    fn converts_rows_to_prepared_with_line_numbers() {
        let rows = vec![
            RowData {
                row_no: 2,
                spelling: "hello".into(),
                phonetic: "/h/".into(),
                pos: "".into(),
                meaning: "你好；喂".into(),
                example: "Hello!".into(),
                tags: "".into(),
            },
            RowData {
                row_no: 3,
                spelling: "".into(),
                phonetic: "".into(),
                pos: "".into(),
                meaning: "".into(),
                example: "".into(),
                tags: "".into(),
            },
            RowData {
                row_no: 4,
                spelling: "bad".into(),
                phonetic: "".into(),
                pos: "".into(),
                meaning: "".into(),
                example: "".into(),
                tags: "".into(),
            },
        ];
        let (prepared, errors) = prepare_rows(&rows);
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].row_no, 2); // 第 2 行
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 4); // 第 4 行（空行在第 3 行被跳过）
        assert_eq!(errors[0].1, "释义不能为空");
    }

    #[test]
    fn prepares_valid_row_as_meaning_item() {
        let rows = vec![RowData {
            row_no: 2,
            spelling: "apple".into(),
            phonetic: "".into(),
            pos: "n.".into(),
            meaning: "苹果".into(),
            example: "".into(),
            tags: "水果；高频".into(),
        }];
        let (prepared, errors) = prepare_rows(&rows);
        assert!(errors.is_empty());
        let row = &prepared[0];
        assert_eq!(row.spelling, "apple");
        assert_eq!(row.pos, "n.");
        assert_eq!(row.meaning, "苹果");
        assert_eq!(row.phonetic, None);
        assert_eq!(row.example, None);
        assert_eq!(row.tag_names, vec!["水果".to_string(), "高频".to_string()]);
    }

    #[test]
    fn groups_rows_by_spelling_merging_meanings() {
        let rows = vec![
            PreparedRow {
                row_no: 2,
                spelling: "apple".into(),
                phonetic: Some("/æpl/".into()),
                pos: "n.".into(),
                meaning: "苹果".into(),
                example: None,
                tag_names: vec!["水果".into()],
            },
            PreparedRow {
                row_no: 3,
                spelling: "apple".into(),
                phonetic: None,
                pos: "v.".into(),
                meaning: "放弃".into(),
                example: Some("Don't apple.".into()),
                tag_names: vec!["高频".into()],
            },
            PreparedRow {
                row_no: 4,
                spelling: "banana".into(),
                phonetic: None,
                pos: "n.".into(),
                meaning: "香蕉".into(),
                example: None,
                tag_names: vec![],
            },
        ];
        let groups = group_rows(&rows);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].spelling, "apple");
        assert_eq!(groups[0].row_nos, vec![2, 3]);
        assert_eq!(groups[0].phonetic.as_deref(), Some("/æpl/"));
        assert_eq!(groups[0].example.as_deref(), Some("Don't apple."));
        assert_eq!(
            groups[0].tag_names,
            vec!["水果".to_string(), "高频".to_string()]
        );
        assert_eq!(groups[0].definitions.len(), 2);
        assert_eq!(groups[0].definitions[0].pos, "n.");
        assert_eq!(groups[0].definitions[0].meaning, "苹果");
        assert_eq!(groups[0].definitions[1].pos, "v.");
        assert_eq!(groups[1].spelling, "banana");
        assert_eq!(groups[1].definitions.len(), 1);
    }

    #[test]
    fn rejects_over_limit_rows() {
        // 构造 5001 行 csv（含表头）
        let mut csv = String::from("单词,音标,词性,释义,例句,标签\n");
        for _ in 0..=5000 {
            csv.push_str("word,,,x,,\n");
        }
        let err = parse_csv(csv.as_bytes(), 5000).unwrap_err();
        assert!(matches!(err, WordError::TooManyRows { .. }));
    }

    #[test]
    fn decodes_gbk_csv() {
        // 用 encoding_rs 生成 GBK 字节（中文 Excel/WPS 另存 CSV 常见编码）
        let (gbk_bytes, _, had_errors) =
            encoding_rs::GBK.encode("单词,音标,词性,释义,例句,标签\nhi,,,你好,,\n");
        assert!(!had_errors);
        let rows = parse_csv(&gbk_bytes, 5000).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spelling, "hi");
        assert_eq!(rows[0].meaning, "你好");
        assert_eq!(HEADERS[0], "单词");
        assert_eq!(HEADERS[5], "标签");
    }

    #[test]
    fn to_dto_roundtrip_preserves_fields() {
        let row = RowData {
            row_no: 7,
            spelling: "apple".into(),
            phonetic: "/æpl/".into(),
            pos: "n.".into(),
            meaning: "苹果".into(),
            example: "An apple.".into(),
            tags: "水果".into(),
        };
        let dto = row.to_dto();
        assert_eq!(dto.row, 7);
        let back = RowData::from_dto(&dto);
        assert_eq!(back.row_no, 7);
        assert_eq!(back.spelling, "apple");
        assert_eq!(back.tags, "水果");
        assert!(!RowData::from_dto(&dto).is_blank());
    }
}
