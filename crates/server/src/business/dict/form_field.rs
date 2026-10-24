/// 表单字段（值通常来自查询词，故带生命周期；名字为静态字面量）。
///
/// 序列化交给 `FormBody`（`reqwest` 的表单要求顶层是映射）。
#[derive(Debug, Clone, Copy)]
pub struct FormField<'a> {
    pub name: &'static str,
    pub value: &'a str,
}

impl<'a> FormField<'a> {
    pub const fn new(name: &'static str, value: &'a str) -> Self {
        Self { name, value }
    }
}
