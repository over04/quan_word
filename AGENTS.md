# 单词本开发约定

- 后端 Rust：Axum + SeaORM，SQLite 默认、支持 PostgreSQL。业务目录对应 URL，按 router → service → repo 分层；模块用 `foo.rs`，不写 `mod.rs` 或 `pub use`，通常每文件一个类型。HTTP 输入用 typed DTO/枚举、`ApiJson`/`ApiPath`，错误返回中文 JSON。
- 前端 React + TypeScript + Tailwind：页面管理状态，`frontend/src/api.ts` 统一请求。契约由 ts-rs 生成，禁止手改 `frontend/src/generated/`；改 DTO 后跑 Rust 测试和前端构建。UI 用中文，沿用现有视觉和动效。
- Compose 将 `config.yaml` 只读挂载到 `/app/config.yaml`，数据在 `./data`。显式配置路径缺失即停止启动；对外部署设置非空、无空格的可见 ASCII 密钥。浏览器先验证再保存，只有当前密钥的 401 能清除会话。
- CSV 按记录起始物理行修正；导入只为实际写入的词建标签，标签和词同事务。词典插件可独立装卸，缺少配置中列出的插件只记警告；查询按启用源顺序，启停/装卸使相关缓存失效。
- 验证：`cargo fmt --check`、`cargo clippy --workspace --all-targets --all-features -- -D warnings`、`cargo test --workspace`、`cargo run -p xtask -- validate-plugins`；前端运行 `npm test`、`npm run build`。改数据库迁移时验证 SQLite 和 PostgreSQL；生产构建先构建前端再编译后端。
- 未获用户明确授权，不提交 Git。
