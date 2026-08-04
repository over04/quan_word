use strum::EnumString;

/// 词性白名单（21 种书写形式 → 18 个语义变体；c/C、u/U、cu/CU 各为同义书写）。
///
/// 与前端词性下拉一致；校验时对释义 `pos` 字段 `trim` 后匹配，留空合法
/// （空值判断在调用方提前处理）。映射由 `strum::EnumString` 声明，业务层
/// 不再维护字符串白名单数组。
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum WordPos {
    #[strum(serialize = "n.")]
    N,
    /// 可数名词（c / C）
    #[strum(serialize = "c", serialize = "C")]
    Countable,
    /// 不可数名词（u / U）
    #[strum(serialize = "u", serialize = "U")]
    Uncountable,
    /// 可数/不可数兼用（cu / CU）
    #[strum(serialize = "cu", serialize = "CU")]
    Both,
    #[strum(serialize = "v.")]
    V,
    #[strum(serialize = "vt.")]
    Vt,
    #[strum(serialize = "vi.")]
    Vi,
    #[strum(serialize = "adj.")]
    Adj,
    #[strum(serialize = "adv.")]
    Adv,
    #[strum(serialize = "prep.")]
    Prep,
    #[strum(serialize = "conj.")]
    Conj,
    #[strum(serialize = "pron.")]
    Pron,
    #[strum(serialize = "num.")]
    Num,
    #[strum(serialize = "art.")]
    Art,
    #[strum(serialize = "interj.")]
    Interj,
    #[strum(serialize = "aux.")]
    Aux,
    #[strum(serialize = "abbr.")]
    Abbr,
    #[strum(serialize = "phr.")]
    Phr,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::WordPos;

    /// 21 种书写形式全部可解析（与前端下拉、旧白名单数组一致）。
    #[test]
    fn parses_all_21_written_forms() {
        for raw in [
            "n.", "c", "C", "u", "U", "cu", "CU", "v.", "vt.", "vi.", "adj.", "adv.", "prep.",
            "conj.", "pron.", "num.", "art.", "interj.", "aux.", "abbr.", "phr.",
        ] {
            assert!(WordPos::from_str(raw).is_ok(), "词性书写形式应合法: {raw}");
        }
    }

    #[test]
    fn rejects_unknown_or_mixed_case_pos() {
        // 与旧白名单一致：大小写混用与未收录形式非法
        for raw in ["Cu", "cU", "N", "V", "x.", "noun", "", " "] {
            assert!(WordPos::from_str(raw).is_err(), "词性应非法: {raw:?}");
        }
    }

    #[test]
    fn maps_synonymous_written_forms_to_same_variant() {
        assert_eq!(
            WordPos::from_str("c").unwrap(),
            WordPos::from_str("C").unwrap()
        );
        assert_eq!(
            WordPos::from_str("u").unwrap(),
            WordPos::from_str("U").unwrap()
        );
        assert_eq!(
            WordPos::from_str("cu").unwrap(),
            WordPos::from_str("CU").unwrap()
        );
    }
}
