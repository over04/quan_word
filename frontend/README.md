# Frontend

单词本（quan_word）前端：React 19 + Vite 8 + TailwindCSS v4。

```bash
npm install
npm run dev       # 开发服务器 :5173，/api 代理到 :3000
npm run build     # tsc -b + vite build → dist/（rust-embed 编译期嵌入，需在 cargo build 前执行）
```

## 目录

- `src/api.ts` — fetch 封装，契约类型从 `src/generated/` 转发（ts-rs 生成，勿手改）
- `src/generated/` — 后端 ts-rs 导出的共享契约类型；后端 `cargo test` 时刷新
- `src/pages/`、`src/components/` — 页面与组件
- `public/fonts/` — 自托管字体（无 Google Fonts CDN）

设计系统见 `../docs/dev/style.md`。
