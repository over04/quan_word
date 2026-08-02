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
│   ├── entity/             SeaORM 实体 + Definition（definitions JSON 列模型）
│   └── migration/          数据库迁移
├── frontend/               前端 SPA（src/generated/ 为 ts-rs 生成契约类型）
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
cd frontend && npm run build   # tsc -b + vite build
```

### 配置

复制 `config.example.yaml` 为 `config.yaml`（已 git 忽略）后修改；或通过环境变量 `QUAN_WORD_CONFIG` 指定路径。切换 PostgreSQL 只需改 `database.url`。

### 前后端契约

共享边界类型由后端 ts-rs 导出到 `frontend/src/generated/`（`cargo test` 时刷新）。该目录为契约唯一事实来源，**勿手改**；修改 DTO 后重新 `cargo test` 并提交生成文件。

## API 一览

| 方法 | 路径 | 说明 |
|---|---|---|
| GET/POST | `/api/wordbooks` | 单词书列表 / 创建 |
| GET/PUT/DELETE | `/api/wordbooks/{id}` | 单书 / 更新 / 删除（级联删词） |
| GET/POST | `/api/wordbooks/{book_id}/words` | 分页浏览（order: id_asc/id_desc/spelling/random，random 需 seed）/ 添加单词 |
| GET | `/api/wordbooks/{book_id}/words/query` | 列表模式：搜索 + 排序（sort/order）+ 分页 |
| PUT/DELETE | `/api/wordbooks/{book_id}/words/{id}` | 更新 / 删除单词（校验归属该书） |

分页参数：`page`（1 起）、`page_size`（默认 20，上限 100）。错误响应为 `{"error": "中文消息"}`。

## 部署

```bash
cd frontend && npm run build        # 先构建前端
cd .. && cargo build --release      # 产物 ./target/release/server
./target/release/server             # 从项目根运行（读 config.yaml）
```

## 设计

UI 遵循 **Visual Organic** 设计系统（`docs/dev/style.md`）：象牙白/炭黑/陶土/鼠尾草绿/沙色配色，DM Serif Display + Plus Jakarta Sans 字体，无 emoji、无渐变。
