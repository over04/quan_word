# 圈圈单词（quan_word）

纸质笔记本风格的单词学习 Web 应用。把单词整理进独立的"单词书"，在模拟纸质书视图中复习（横线格、点击遮罩自测），在列表视图中管理。

## 技术栈

| 层 | 选型 |
|---|---|
| 后端 | Rust：axum 0.8 + SeaORM 2.0 + sea-orm-migration |
| 前端 | React 19 + Vite 8 + TailwindCSS v4（TypeScript） |
| 数据库 | SQLite（默认）/ PostgreSQL（YAML 配置切换） |
| 部署 | 单二进制：rust-embed 编译期嵌入前端产物 |

## 目录结构

```
├── crates/
│   ├── server/             后端应用：business（URL 对齐的业务域）/ common / config / init
│   ├── lexicon/            Definition / WordPos 共享模型（前后端契约）
│   ├── dict-core/          可移植词典内核：插件注册表 + 配方引擎 + SearchTransport
│   ├── dict-reqwest/       reqwest 传输层（超时 / 重定向白名单 / 响应上限）
│   ├── entity/             SeaORM 实体
│   ├── migration/          数据库迁移
│   └── xtask/              仓库维护：示例插件校验
├── examples/               示例插件、fixtures、goldens（不进运行时）
├── plugins/                运行时插件目录（默认为空，从设置页导入）
├── frontend/               前端 SPA（src/generated/ 为 ts-rs 生成契约类型）
├── Dockerfile              多阶段构建：node → rust(musl 静态) → scratch（约 20MB）
├── docker-compose.yml      一键容器化部署（数据挂载 ./data/）
├── .dockerignore
├── config.example.yaml     运行配置模板（复制为 config.yaml 使用，已 git 忽略）
└── AGENTS.md               仓库开发约定（开发前必读）
```

后端业务目录仿照 API URL 结构组织，路由在各层 `router.rs` 注册并递归聚合：

```
business/
└── wordbooks/                        /api/wordbooks...
    ├── router.rs                    注册本层路由 + 聚合子域
    └── words/                        /api/wordbooks/{book_id}/words...
```

## 快速开始

```bash
# 1. 构建前端（rust-embed 编译期需要产物；之后改前端需重新执行）
cd frontend && npm install && npm run build && cd ..

# 2. 准备配置（可选，缺省使用内建默认：sqlite + 0.0.0.0:3000）
cp config.example.yaml config.yaml

# 3. 启动
cargo run -p server        # 访问 http://localhost:3000
```

## 开发

```bash
cargo run -p server            # 后端 :3000（根目录运行，读 config.yaml）
cd frontend && npm run dev     # 前端热更新 :5173，/api 代理到 :3000

# 验证门槛
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace         # 单元测试 + ts-rs 契约导出（刷新 frontend/src/generated/）
cargo fmt --all --check
cargo run -p xtask -- validate-plugins   # 校验 examples/plugins 示例包
cd frontend && npm run test && npm run build
```

### 配置

复制 `config.example.yaml` 为 `config.yaml`（已 git 忽略）后修改；或通过环境变量 `QUAN_WORD_CONFIG` 指定路径。切换 PostgreSQL 只需改 `database.url`。

### 前后端契约

共享边界类型由后端 ts-rs 导出到 `frontend/src/generated/`（`cargo test` 时刷新）。该目录为契约唯一事实来源，**勿手改**；修改 DTO 后重新 `cargo test` 并提交生成文件。

### 词典插件

词典抓取是**运行时装载的声明式配方插件**，不是编译进二进制的 Rust 模块：

- 运行时只从 `plugins/*.json` 装载，仓库不内置任何源；8 个示例包在 `examples/plugins/`，仅用于测试与手动导入；
- 设置页「词典源」只做列表、启停、导入、删除；目录默认 5 秒轮询，文件增删自动重载；
- 导入 = multipart 上传 JSON，服务端重新校验后写 `plugins/<id>.json` 并立即生效；删除会移除对应文件；
- 配方支持 JSON/HTML 提取、变量、变换、条件、释义策略与建议列表，安全上限（主机白名单、包体/响应大小、条数）由内核强制；
- 完整格式、字段、调试与移动端嵌入见 [`docs/dev/plugins.md`](docs/dev/plugins.md)。

```bash
cargo run -p xtask -- validate-plugins            # 校验 examples/plugins
cargo run -p xtask -- validate-plugins ./plugins  # 校验运行时目录
```

## API 一览

| 方法 | 路径 | 说明 |
|---|---|---|
| GET/POST | `/api/wordbooks` | 单词书列表 / 创建 |
| GET/PUT/DELETE | `/api/wordbooks/{id}` | 单书 / 更新 / 删除（级联删词） |
| GET/POST | `/api/wordbooks/{book_id}/words` | 分页浏览（order: id_asc/id_desc/spelling/random，random 需 seed）/ 添加单词 |
| GET | `/api/wordbooks/{book_id}/words/query` | 列表模式：搜索 + 排序（sort/order）+ 分页 |
| PUT/DELETE | `/api/wordbooks/{book_id}/words/{id}` | 更新 / 删除单词（校验归属该书） |
| GET | `/api/dict/providers` | 已安装词典插件（启用状态、版本、能力） |
| POST | `/api/dict/plugins` | 上传插件 JSON（multipart 字段 `file`），校验后立即装载 |
| DELETE | `/api/dict/plugins/{id}` | 删除插件文件并立即卸载 |
| PUT | `/api/dict/plugins/{id}/enabled` | 运行时启停插件（持久化到 `.state.json`） |
| GET | `/api/dict/lookup` | 聚合抓词：`word` + 可选 `providers`（逗号分隔）+ `refresh=1` |
| GET | `/api/dict/suggest` | 模糊建议（按 IP 限流） |

分页参数：`page`（1 起）、`page_size`（默认 20，上限 100）。错误响应为 `{"error": "中文消息"}`。配置 `server.auth_key` 后所有 `/api` 请求必须带 `Authorization: Bearer <key>` 或 `X-Api-Key`；未配置时整个 `/api` 不启用鉴权，属于部署特例。

## 部署

### Docker（推荐）

```bash
docker compose up -d --build   # 构建并启动，访问 http://localhost:3000
```

- 镜像约 20MB（scratch + musl 静态链接二进制，仅含可执行文件与 CA 证书）
- 构建缓存：cargo registry 与 target 挂 BuildKit cache mount，依赖未变时改码重建约 1 分钟
- 数据持久化在 `./data/`（SQLite），可直接 `sqlite3 data/quan_word.db` 查看/备份
- 自定义配置（auth_key / 切换 PostgreSQL）见 docker-compose.yml 内注释

### 本地构建

```bash
cd frontend && npm run build        # 先构建前端
cd .. && cargo build --release      # 产物 ./target/release/server
./target/release/server             # 从项目根运行（读 config.yaml）
```

## 设计

UI 遵循 **Visual Organic** 设计系统（`docs/dev/style.md`）：象牙白/炭黑/陶土/鼠尾草绿/沙色配色，DM Serif Display + Plus Jakarta Sans 字体，无 emoji、无渐变。
