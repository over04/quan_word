//! 业务域：目录层级仿照 API URL 结构（`/api/wordbooks/...` → `wordbooks/`，子资源 `words` 为子域），
//! 各层 `router.rs` 注册本层路由并聚合下层，顶层 `router.rs` 只做入口聚合。

pub mod wordbooks;
