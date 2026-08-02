use rust_embed::RustEmbed;

/// 编译期嵌入的前端构建产物（frontend/dist）。
#[derive(RustEmbed)]
#[folder = "../../frontend/dist/"]
pub struct Asset;
