# 词典插件开发

词典抓取不再编译进服务端：每个源是一个 **声明式配方（recipe）插件**，运行时只从 `plugins/*.json` 装载，仓库不内置任何源。新增/修改源只需要写 JSON，不改 Rust、不重新编译；同一套内核也能在 iOS / Android / WASI 上运行。

`examples/plugins/` 存放 8 个示例包，仅用于测试与手动导入；服务启动时 `plugins/` 为空是合法状态。

## 分层

| crate | 职责 | 依赖 |
|---|---|---|
| `lexicon` | `Definition` / `WordPos` 共享模型 | serde |
| `dict-core` | 插件模型、注册表、配方引擎、`SearchTransport` trait | serde / serde_json / scraper / url（无 reqwest、无 tokio、无 std::net） |
| `dict-reqwest` | `ReqwestTransport`：HTTP、超时、重定向白名单、响应大小限制 | reqwest(rustls) |
| `server` | `SearchEngine` + 缓存/熔断/限流 + HTTP 接口 + 插件目录管理 | 以上全部 |
| `examples/` | 8 个示例插件、fixtures、goldens；不进运行时 | — |

`dict-core` 只通过 `SearchTransport` 触网：

```rust
#[async_trait]
pub trait SearchTransport {
    async fn execute(&self, request: SearchRequest) -> Result<SearchResponse, TransportError>;
}
```

移动端/宿主只需实现这个 trait（URLSession、OkHttp、fetch 等），内核与插件无需改动。

## 插件包格式

`plugins/<id>.json`：

```json
{
  "manifest": {
    "api_version": 1,
    "id": "example",
    "label": "示例词典",
    "short_label": "示例",
    "description": "一句话说明",
    "version": "1.0.0",
    "engine": "recipe",
    "capabilities": ["lookup", "suggest"],
    "hosts": ["dict.example.com"],
    "limits": {
      "max_body_bytes": 1048576,
      "max_definitions": 20,
      "max_examples": 5
    }
  },
  "recipe": {
    "lookup": { "...": "见下" },
    "suggest": { "...": "见下" }
  }
}
```

清单字段：

| 字段 | 必填 | 说明 |
|---|---|---|
| `api_version` | 是 | 当前只支持 `1` |
| `id` | 是 | 小写字母开头，仅 `[a-z0-9_-]`，≤32 字符；同时是缓存 key 的一部分 |
| `label` | 是 | 中文/展示全名，≤64 字符 |
| `short_label` | 否 | 窄位短名，≤16 字符；缺省用 `label` |
| `description` | 否 | ≤200 字符 |
| `version` | 是 | ≤32 字符；**改解析逻辑必须升版本**，否则旧缓存不会失效 |
| `engine` | 是 | 目前只支持 `"recipe"`（`"wasm"` 预留给未来） |
| `capabilities` | 否 | `lookup` / `suggest`，缺省 `["lookup"]`；声明什么就必须提供对应配方 |
| `hosts` | 是 | 允许访问的主机白名单，支持精确主机、`*.example.com`、`host:port`；重定向后的最终 host 也会校验 |
| `limits` | 否 | 只能比引擎硬上限更小；缺省 1 MiB / 20 条释义 / 5 条例句 |

安全限制（引擎硬上限，插件不能突破）：包体 ≤256 KiB、响应 ≤1 MiB、释义 ≤20、例句 ≤5、建议 ≤24、单条释义 ≤300 字符、最多 32 个插件；URL 只允许 `http/https`。

## 配方基础

### 请求

```json
{
  "method": "get",
  "url": "https://dict.example.com/api?q={word}",
  "headers": [{ "name": "Accept-Language", "value": "zh-CN" }],
  "form": [{ "name": "q", "value": "{word}" }]
}
```

- `{word}` 会被 URL 编码后替换；出现其它 `{...}` 视为非法。
- `method` 支持 `get` / `post`；`post` 时 `form` 按 `application/x-www-form-urlencoded` 发送。
- 请求头值不允许换行，也不允许包含 `{`。

### 提取器（Extractor）

配方里所有取值位置都用提取器，JSON 与 HTML 共用：

| 写法 | 含义 |
|---|---|
| `{ "path": "a.b[0].c" }` | JSON 路径，取第一个值；支持 `[*]` 数组展开、`[n]` 下标 |
| `{ "all": "data[*].text" }` | JSON 路径，取全部值 |
| `{ "selector": "meta[name=\"description\"]", "attr": "content" }` | CSS 选择器；带 `attr` 取属性，否则取元素文本；`"all": true` 取全部匹配 |
| `{ "var": "name" }` | 前面 `vars` 定义的变量 |
| `{ "query": true }` | 用户查询词原文 |
| `{ "text": "固定值" }` | 常量 |
| `{ "template": "https://x.com/{word}" }` | 模板；`{word}` 会被 URL 编码 |
| `{ "first": [ ... ] }` | 按顺序取第一个非空结果 |
| `{ "prefer": { "from": ..., "field": "word", "equals": "query", "ignore_case": true, "fallback": "first" } }` | 优先取与查询词相等的节点，否则回退第一个/空 |
| `{ "field": "key", "from": ... }` | 从上游结果对象里取字段 |

### 变换（Transform）

数组按顺序执行；字符串是单位变换，单键对象是带参变换。

单位变换：`clean_html`、`collapse_ws`、`trim`、`normalize_phonetic`、`map_english_pos`、`canonical_pos`、`lowercase`、`url_encode`。

带参变换：

| 写法 | 说明 |
|---|---|
| `{ "join": "\n" }` | 把多个值拼成一个 |
| `{ "split": [";", "；"] }` | 按分隔符切成多个值 |
| `{ "take": 3 }` | 只保留前 N 个 |
| `{ "dedup": "word_lowercase" }` | 按小写单词去重（建议列表用） |
| `{ "strip_chars": ["[", "]"] }` | 删除指定字符 |
| `{ "between": ["美[", "]"] }` | 取两标记之间的内容；找不到时保留原值 |
| `{ "before": "网络释义" }` | 取标记之前；找不到时保留原值 |
| `{ "after": "网络释义" }` | 取标记之后；找不到时变空 |
| `{ "after_last": "]" }` | 取最后一个标记之后；找不到时保留原值 |
| `{ "trim_chars": "，,：:；; " }` | 删除首尾字符集合 |
| `{ "cap_chars": 300 }` | 按字符数截断 |
| `{ "default": "未提供" }` | 空值时给默认值 |

### lookup 配方

```json
{
  "request": { "method": "get", "url": "https://api.example.com/define?term={word}" },
  "vars": [
    { "name": "phonetic", "path": "data.phonetic", "transform": ["normalize_phonetic"] },
    { "name": "items", "all": "data.definitions[*]", "transform": [{ "take": 3 }] }
  ],
  "word": { "query": true },
  "phonetic": { "first": [{ "var": "phonetic" }] },
  "audio_url": { "path": "data.audio" },
  "source_url": { "template": "https://www.example.com/word/{word}" },
  "definitions": [
    {
      "kind": "items",
      "from": { "var": "items" },
      "pos": { "path": "pos" },
      "pos_transform": ["map_english_pos", "canonical_pos"],
      "meaning": { "path": "meaning" },
      "transform": ["clean_html"]
    }
  ],
  "examples": {
    "from": { "all": "data.examples[*]" },
    "fields": {
      "text": { "path": "sentence" },
      "translation": { "path": "translation" }
    },
    "take": 3,
    "transform": ["clean_html"]
  },
  "empty": "error"
}
```

释义策略（`definitions` 数组按顺序求值，第一个非空生效）：

| `kind` | 用途 | 关键字段 |
|---|---|---|
| `pos_text` | 一段带词性前缀的文本，如 `n. 放弃；v. 抛弃` | `from`、`separators`、`transform` |
| `plain_text` | 一段无词性文本，按分隔符切 | `from`、`separators` |
| `items` | 节点列表，每个节点一个义项 | `from`、`pos`、`meaning`、`split` |
| `groups` | 分组节点，组内再取义项 | `from`、`pos`、`items` |

所有策略都支持 `when` 条件：

- `{ "non_empty": "var_name" }`
- `{ "empty": "var_name" }`
- `{ "contains": { "var": "var_name", "value": "关键字" } }`

`empty` 控制“成功响应但没有解析到释义”：

- `"error"`（缺省）：返回“响应解析失败”；
- `"not_found"`：按未收录处理（写负缓存）。

### suggest 配方

```json
{
  "request": { "method": "get", "url": "https://api.example.com/suggest?q={word}" },
  "items": {
    "from": { "all": "data[*]" },
    "fields": {
      "word": { "path": "word" },
      "summary": { "path": "meaning" }
    },
    "transform": ["trim"]
  },
  "take": 24,
  "dedup": "word_lowercase"
}
```

建议接口只使用第一个“启用且声明了 `suggest`”的插件；失败或空结果继续下一个，全部失败返回空数组（不报错）。

## 装载

1. 只读 `dict.plugins_dir`（默认 `./plugins`）下的 `*.json`，按文件名排序；仓库不内置任何源。
2. 目录不存在或为空时服务照常启动，词典接口返回“没有可用的词典源”；从设置页导入后立即可用。
3. 两个包同 id：两个都拒绝（不会“后写覆盖先写”），拒绝原因进启动日志。
4. 超过 `dict.max_plugins` 的包被拒绝。
5. 解析失败/主机越界/能力与配方不一致的包只影响自己，其它包照常装载。

`dict.providers` 为空表示启用全部已安装插件；非空时按该数组顺序启用并聚合，未知 id 启动报错。`providers` 只影响“启用/顺序”，不阻止安装；设置页的运行时启停覆盖它并持久化到 `plugins/.state.json`。

## 自动发现、导入与删除

- 启动时装载一次。
- `dict.plugins_poll_secs > 0`（默认 5 秒）时后台扫描目录；文件名、长度、mtime 都没变就不重载，避免无谓清缓存。
- `POST /api/dict/plugins`：multipart 字段 `file`，上传 JSON 包；服务端重新校验，写为 `plugins/<id>.json`，删除同 id 的旧外部文件，然后立即重载。请求体上限 300 KiB。
- `DELETE /api/dict/plugins/{id}`：删除该 id 的插件文件并立即重载。
- `PUT /api/dict/plugins/{id}/enabled`：运行时启停，写 `.state.json`，不移动文件。
- `GET /api/dict/plugins/{id}`：只读详情蓝图，返回 manifest 能力/hosts、lookup/suggest 请求、注册字段、释义策略、变量、插件上限与引擎硬上限；不返回原始 JSON。未知 id 返回 404。
- 没有“重新加载”接口：文件变化由轮询接管，导入/删除由接口内部重载。

设置页列表只保留插件行、开关、导入、删除；点击行进入详情蓝图（能力、请求、注册字段、释义策略、变量、建议、上限环），不展示目录、统计、装载报告或原始 JSON。

## 新增一个源

1. 在 `examples/plugins/` 新建 `<id>.json`（示例/测试用），或直接在设置页导入到运行时 `plugins/`。
2. 在 `examples/fixtures/` 放一份真实响应，在 `crates/dict-core/tests/examples.rs` 加回归断言（或复用现有 golden）。
3. 跑校验：

```bash
cargo run -p xtask -- validate-plugins
cargo test -p dict-core
```

4. 启动服务后从设置页导入，或：

```bash
curl -s -F 'file=@examples/plugins/<id>.json' localhost:3000/api/dict/plugins | jq
curl -s 'localhost:3000/api/dict/plugins/<id>' | jq
curl -s 'localhost:3000/api/dict/lookup?word=abandon&providers=<id>' | jq
```

若配置了 `server.auth_key`，所有 `/api` 请求（包括导入/删除/建议）都要带 `Authorization: Bearer <key>`；未配置时整个 `/api` 不启用鉴权，这是部署特例。

## 调试

- 解析失败常见原因：JSON 路径写错（数组要写 `[*]`）、`hosts` 没覆盖实际域名/重定向域名、声明了 `lookup` 能力却没有 `recipe.lookup`、`version` 没升导致旧缓存。
- 启动日志会打印 loaded/rejected 数量；单个文件的失败原因在日志里。
- 单元测试用 `MockTransport` 注入固定响应，不访问网络（见 `crates/server/src/business/dict/service.rs` 的测试）；示例包回归在 `crates/dict-core/tests/examples.rs`。

## 在移动端嵌入

`dict-core` 不依赖 reqwest/tokio/文件系统，宿主只需要：

```rust
use std::sync::Arc;
use dict_core::engine_limits::EngineLimits;
use dict_core::provider_id::ProviderId;
use dict_core::search_engine::SearchEngine;
use dict_core::transport::SearchTransport;

let engine = SearchEngine::new(Arc::new(MyPlatformTransport), EngineLimits::default());
engine.load(host_sources); // 宿主从 App 沙盒读取的 PluginSource 列表

let entry = engine.lookup(&ProviderId::new("youdao")?, "abandon").await?;
```

平台传输层负责把 `SearchRequest`（方法、URL、头、表单）转成 URLSession/OkHttp/fetch 调用，并返回 `SearchResponse { status, body, final_url }`；重定向后的 host 校验由传输层或 `dict-core` 的 `host_allowed` 完成。CI 的 `cross` job 会对 `aarch64-apple-ios`、`aarch64-linux-android`、`wasm32-wasip1` 做 `cargo check`，防止内核引入平台相关依赖。
