use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use super::form_field::FormField;

/// 表单请求体：把 `FormField` 列表序列化为 `name → value` 映射。
///
/// `reqwest` 的 `.form()` 走 `serde_urlencoded`，顶层必须是 map/struct；
/// 直接把 `FormField` 列表交给它会被当作「序列的序列」而 builder 失败。
#[derive(Debug, Clone, Copy)]
pub struct FormBody<'a> {
    fields: &'a [FormField<'a>],
}

impl<'a> FormBody<'a> {
    pub const fn new(fields: &'a [FormField<'a>]) -> Self {
        Self { fields }
    }
}

impl Serialize for FormBody<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        for field in self.fields {
            map.serialize_entry(field.name, field.value)?;
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::FormBody;
    use crate::business::dict::form_field::FormField;

    #[test]
    fn serializes_as_named_map() {
        let fields = [
            FormField::new("kw", "abandon"),
            FormField::new("lang", "en"),
        ];
        let value = serde_json::to_value(FormBody::new(&fields)).unwrap();
        // 顶层必须是对象（映射），否则 reqwest 的 form 编码会失败
        assert_eq!(value, serde_json::json!({ "kw": "abandon", "lang": "en" }));
    }
}
