# 设计规范（Visual Organic）

本规范基于参考页 https://www.uiprompt.site/zh/styles/preview/visual-organic 提取，是全项目唯一的视觉依据。实现对照 `frontend/src/index.css`（`@theme` 色板与 keyframes）与各组件。

## 核心视觉特征

**配色（Tailwind v4 `@theme` 自定义色板）**：

| Token | 色值 | 用途 |
|---|---|---|
| `ivory` | `#F8F4EF` | 页面背景（纯色，**禁止渐变**） |
| `charcoal` | `#2F2A25` | 主文字、主按钮背景、深色封面 |
| `clay` | `#C58F6D` | 强调色：关键词、进度条、焦点环、装饰圆点 |
| `sage` | `#C9D5C6` | 品牌点缀：logo 圆形底、图标底 |
| `sand` | `#E5D8C8` | 浅色块：徽章底、按钮 hover 底 |

**字体**：
- 标题：`font-serif` = **DM Serif Display**（拉丁）+ `Songti SC / Noto Serif SC / SimSun`（中文回退）
- 正文/UI：`font-sans` = **Plus Jakarta Sans** + 中文无衬线回退
- 字距 `0.015em`，行高 1.7（全局 body）

**禁止项（用户强制）**：
- 界面**不得出现 emoji**——图标一律用 `frontend/src/components/Icons.tsx` 的线性 SVG（stroke 1.8、圆角端点）
- 背景**不得使用渐变**——纯色平铺（页面 `bg-ivory`，卡片 `bg-white`）
- 不得放置"操作提示"类帮助文字在前端界面

## 组件规范

**悬浮胶囊导航**（列表页/详情页共用）：
```
bg-white/70 backdrop-blur-md border border-white/40 shadow-sm rounded-full px-6 py-3
```
- Logo：`w-8 h-8 bg-sage rounded-full` 圆形图标 + `font-serif` 品牌名（强调字用 `text-clay`）

**按钮**：
- 主按钮：`bg-charcoal text-ivory rounded-full font-medium shadow-lg shadow-charcoal/10 hover:bg-charcoal/90`
- 次按钮：`border border-charcoal/20 rounded-full text-charcoal hover:bg-white hover:border-charcoal/40`
- 文字色次级用 `text-charcoal/70`，弱化用 `text-charcoal/40`

**徽章**：`bg-sand/30 border border-sand rounded-full text-xs font-bold tracking-wide text-charcoal/70` + `w-2 h-2 bg-clay rounded-full` 圆点

**卡片**：`bg-white rounded-2xl shadow-sm border border-charcoal/5`（列表页封面卡、表格容器、纸页）；hover 用 `card-hover`（上浮 2px + `0 12px 32px rgba(47,42,37,0.1)`）

**装饰**：
- 有机 blob：`bg-sage/30 blur-2xl` + `blob-shape`（border-radius 60% 40% 30% 70% / 60% 30% 70% 40%），仅作页面背景点缀
- 颗粒纹理：`.texture-overlay`（fixed 全屏、feTurbulence、opacity 0.04、pointer-events-none）
- 曲线分隔：`WavyDivider`（SVG path 波浪线，`text-clay/35`）

**焦点/可访问性**：
- 全局 `*:focus-visible { outline: 2px solid #C58F6D; outline-offset: 4px }`
- `prefers-reduced-motion` 下关闭全部动画

## 纸质书模式（PaperBookView）

- **纸张**：`bg-[#FDFAF4] rounded-2xl shadow-sm border-charcoal/5`，宽度 `w-full` 随屏幕；下层垫一张 `bg-[#F2EAE0] translate-x-2 translate-y-2` 模拟纸堆
- **装订线**：左侧 `w-px bg-[#E9B8BC]/70`，内容区整体右移避开（grid `px-9 md:px-11`）
- **行结构**：每条横线（`h-px bg-[#E7DAC6]`）对应一行；**单词在横线上方**（底部贴线）、**释义在横线下方**；一行可容纳多个单词（`grid-cols-2 sm:3 lg:4 2xl:5` 随屏）
- **遮挡自测**：点单词/释义 → 深炭墨条（`bg-charcoal text-transparent` + `shadow-[inset_0_2px_4px_rgba(0,0,0,0.3)]`），保留原文宽度零跳动；按钮带 `aria-pressed`
- **翻页**（电子书式，无按钮）：点击纸面左/右 1/3 热区翻页（hover 显示半透明 chevron）；按住拖动拟真跟随（`translateX + rotateY`，阈值 90px）；键盘 ←/→；翻页动画 `pageIn`（perspective 1600px，从右侧翻入）+ 翻走态（左移 16% + rotateY 8deg + opacity 0.45）
- **页脚**：细进度条（`h-0.5 bg-charcoal/10`，填充 `bg-clay`）+ 页码（`第 X 页 · 共 Y 页`），无操作提示文字

## 列表模式（WordTable）

- 表格容器：`bg-white rounded-2xl shadow-sm border-charcoal/5`
- 表头：`text-xs font-bold text-charcoal/40 uppercase tracking-widest`，行分隔 `border-charcoal/5`，行 hover `bg-sand/20`
- 移动端自动切换卡片列表（`md:hidden divide-y divide-charcoal/5`）

## 弹窗（Word/Wordbook FormModal）

- 遮罩：`bg-charcoal/25 backdrop-blur-sm`，面板 `bg-white rounded-[2rem] border-charcoal/5 shadow-2xl shadow-charcoal/15`
- 标题 `font-serif text-xl` + 圆形图标底（`bg-sage`，SVG 图标）
- 输入：`bg-white border-charcoal/15 text-sm focus:border-clay focus:outline-none`
- 词性必须用下拉枚举（`POS_OPTIONS` 13 项，空值占位灰色 `text-charcoal/35` 且 `disabled hidden` 不出现在列表）；后端 `word_service.rs` 同步校验

## 阅读设置（SettingsPanel）

- 每页单词数：滑块 10–200（step 10）；字号：滑块 12–28px（单词字号，音标 ×0.55、释义 ×0.6、行高 ×1.7 按比例联动）
- 滑块拖动中仅更新显示值（draft state），`onPointerUp`/`onKeyUp` 才应用；持久化 `localStorage`（`qw_page_size` / `qw_font_scale`）

## 动画

```css
@keyframes morph     /* blob 形变，仅背景装饰 */
@keyframes fadeInUp  /* 0.8s cubic-bezier(0.2,0.8,0.2,1)，弹窗/区块进入 */
@keyframes fadeIn    /* 0.6s，遮罩 */
@keyframes pageIn    /* 0.4s，翻页进入：translateX(16%) rotateY(-9deg) → 0 */
```
`prefers-reduced-motion: reduce` 下全部禁用。

## 响应式

- 纸张/内容容器：`max-w-7xl mx-auto px-4 md:px-8`（详情页），随视口变化
- 纸质书列数：`grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5`
- 卡片网格：`grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8`
