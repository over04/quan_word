/// 待装载的插件包：文件名 + 原始字节（UTF-8 JSON）。
#[derive(Debug, Clone)]
pub struct PluginSource {
    pub file: String,
    pub bytes: Vec<u8>,
}

impl PluginSource {
    pub fn new(file: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            file: file.into(),
            bytes,
        }
    }
}
