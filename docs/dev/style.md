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

**悬浮胶囊导航**（主页 `HomeNav.tsx`，类名 `.home-nav*`）：
```
nav.home-nav fixed top-0 z-40（padding-top 让位 safe-area）
  .home-nav-card rounded-[1.75rem] md:rounded-full border border-white/60
  背景 rgb(255 255 255 / 0.88)（桌面 0.82）+ 内高光 0 1px 0 rgb(255 255 255 / .9) inset
  投影 0 12px 34px -16px rgb(47 42 37 / .28)
```
- **不用 `backdrop-blur`**：固定导航在滚动时每帧都要重新采样并模糊背景，是移动端最贵的一笔开销；改用高不透明度白 + 内高光 + 柔和投影静态还原玻璃感，滚动过程零重绘。
- Logo：`w-9 h-9 md:w-8 md:h-8 bg-sage rounded-full` 圆形图标 + `font-serif` 品牌名（强调字用 `text-clay italic`），hover 时旋转 -9° 并放大 1.07。
- 分区切换：分段控件 `.nav-seg`（grid 两等分）+ 绝对定位白色滑块 `.nav-seg-pill`（`translate3d(index*100%,0,0)`，0.46s `cubic-bezier(.34,1.42,.64,1)`），语义 `role="tablist"` / `role="tab"`，键盘 ←/→ 切换。
- 触控：移动端品牌 120×46、tab 155×46、CTA 126×46，全部 ≥44px；两行布局 nav 高 147px，内容区 `pt-40`（160px）留 13px 余量，导航不遮挡内容。

**按钮**：
- 主按钮：`bg-charcoal text-ivory rounded-full font-medium shadow-lg shadow-charcoal/10 hover:bg-charcoal/90`
- 次按钮：`border border-charcoal/20 rounded-full text-charcoal hover:bg-white hover:border-charcoal/40`
- 文字色次级用 `text-charcoal/70`，弱化用 `text-charcoal/40`

**徽章**：`bg-sand/30 border border-sand rounded-full text-xs font-bold tracking-wide text-charcoal/70` + `w-2 h-2 bg-clay rounded-full` 圆点

**卡片**：`bg-white rounded-2xl shadow-sm border border-charcoal/5`（列表页封面卡、表格容器、纸页）；hover 用各组件自己的 `transition` + 阴影/位移工具类（不要用会拖 `box-shadow` 逐帧重绘的全局过渡）

**装饰**：
- 环境层（`AmbientBackground.tsx`，`createPortal` 到 `<body>`；`.ambient-stage` 在 CSS 里写死 `position: fixed; inset: 0; width/height: 100%; z-index: 0; pointer-events: none; overflow: clip; transform: translateZ(0)`——任何祖先的 transform/filter/contain 都捕获不到它，滚动/合成时纹丝不动；`html` 与 `body` 设 `overscroll-behavior: none`，禁掉下拉刷新与橡皮筋（否则 iOS/Android 下拉会把整页含 fixed 层一起拖走）内含 3 个静态模糊色斑 `.ambient-orb-*`（`filter: blur(44px)`，桌面 72px）、2 个有机轮廓环 `.ambient-halo-*`（`border-radius` 60% 40% 30% 70% / 60% 30% 70% 40%，超慢反向旋转）、6 个漂浮字形 `.ambient-glyph`；色斑不参与动画，动感全部来自 transform/opacity（详见「动画与合成层口径」）
- 颗粒纹理：`.texture-overlay`（fixed 全屏、feTurbulence、opacity 0.04、pointer-events-none）

**焦点/可访问性**：
- 全局 `*:focus-visible { outline: 2px solid #C58F6D; outline-offset: 4px }`；button/a/`[role=button]` 用 3px offset + `border-radius: inherit`，圆角按钮的焦点环贴合外形
- 所有可点区域 ≥44px（触控标准）
- 全局按压反馈：`:where(button:not(:disabled)):active { scale: .96 }`（用独立 `scale` 属性，不覆盖组件的 hover `transform`），移动端关闭系统 tap 高亮
- `prefers-reduced-motion` 下关闭全部动画（含视图过渡）

## 纸质书模式（PaperBookView）

- **纸张**：`bg-[#FDFAF4] rounded-2xl shadow-sm border-charcoal/5`，宽度 `w-full` 随屏幕；下层垫一张 `bg-[#F2EAE0] translate-x-2 translate-y-2` 模拟纸堆
- **装订线**：左侧 `w-px bg-[#E9B8BC]/70`，内容区整体右移避开（grid `px-9 md:px-11`）
- **行结构**：每条横线（`h-px bg-[#E7DAC6]`）对应一行；**单词在横线上方**（底部贴线）、**释义在横线下方**；一行可容纳多个单词（`grid-cols-2 sm:3 lg:4 2xl:5` 随屏）
- **遮挡自测**：点单词/释义 → 文字**模糊晕开**（`blur-[5px]`，保留原文与宽度、零跳动，过渡 200ms）；按钮带 `aria-pressed`
- **翻页**（电子书式，无按钮）：三页轨道（[上一页, 当前页, 下一页]）水平滑动——点击纸面左/右 1/3 热区（hover 显示半透明 chevron）、按住拖动跟手（**滑动过程中相邻页内容直接可见**，阈值 90px 或 18% 宽）、键盘 ←/→；释放过阈值 0.45s 平滑滑到相邻页，数据就位后轨道重挂载对齐（无过渡、无闪烁）
- **页脚**：细进度条（`h-0.5 bg-charcoal/10`，填充 `bg-clay`）+ 页码（`第 X 页 · 共 Y 页`），无操作提示文字

## 列表模式（WordTable）

- 表格容器：`bg-white rounded-2xl shadow-sm border-charcoal/5`
- 表头：`text-xs font-bold text-charcoal/40 uppercase tracking-widest`，行分隔 `border-charcoal/5`，行 hover `bg-sand/20`
- 移动端自动切换卡片列表（`md:hidden divide-y divide-charcoal/5`）

## 弹窗（Word/Wordbook FormModal）

- 遮罩：`bg-charcoal/25 backdrop-blur-sm`，面板 `bg-white rounded-[2rem] border-charcoal/5 shadow-2xl shadow-charcoal/15`
- 标题 `font-serif text-xl` + 圆形图标底（`bg-sage`，SVG 图标）
- 输入：`bg-white border-charcoal/15 text-sm focus:border-clay focus:outline-none`
- 关闭动画：closing 状态给遮罩加 `.modal-exit-overlay`（180ms 淡出）、给面板加 `.modal-exit`（220ms，`scale(.97)` + 下移 8px + 淡出，加速曲线），两者都 `pointer-events: none`，`animationend`（或 ≥240ms 兜底）后再卸载；`prefers-reduced-motion` 下 `animation: none`（元素保持原样直到卸载＝瞬时关闭）
- 词性必须用下拉枚举（`POS_OPTIONS` 13 项，空值占位灰色 `text-charcoal/35` 且 `disabled hidden` 不出现在列表）；后端 `word_service.rs` 同步校验

## 显示设置（SettingsPanel）

- 每页单词数：滑块 10–200（step 10）；字号：滑块 12–28px（单词字号，音标 ×0.55、释义 ×0.6、行高 ×1.7 按比例联动）
- 滑块拖动中仅更新显示值（draft state），`onPointerUp`/`onKeyUp` 才应用；持久化 `localStorage`（`qw_page_size` / `qw_font_scale`）

## 一键模糊（导航栏，纸质书模式）

- 「单词」「释义」两个胶囊按钮（选中态 `bg-charcoal text-ivory shadow-md`，带 `aria-pressed`）——点击整页模糊，再点恢复
- 「单词」按钮同时模糊音标（音标不单独控制）；逐词点击遮挡与一键模糊叠加生效
- 翻页后保持（全局模式，不随翻页清空）

## 动画与合成层口径

**总原则**：动画只碰 `transform` / `opacity` / `scale`（独立属性），不碰 `width/height/top/left/filter/backdrop-filter`。

### 进场动画（`frontend/src/index.css` 的 `@theme`）

| 类 | 时长 / 曲线 | 用途 |
|---|---|---|
| `animate-nav-in` | navIn 0.62s `.16,1,.3,1` | 导航从上方落入 |
| `animate-book-rise` | bookRise 0.6s | 书架卡逐张入场（stagger 80ms，rotateX 9° 抬起） |
| `animate-hero-in` | heroIn 0.52s | 详情页大标题 |
| `animate-detail-in` | detailIn 0.55s | 详情页整体（rotateX 4°） |
| `animate-page-in` | pageIn 0.55s | 主页分区内容 |
| `animate-result-in` | resultIn 0.5s（过冲 `.18,1.14,.4,1`） | 搜索结果行 |
| `animate-step-in` | stepIn 0.44s | 导入步骤切换 |
| `animate-bar-down` / `animate-bar-up` | 0.45s / 0.5s | 详情页顶栏 / 底栏 |
| `animate-fade-in-up` / `animate-fade-in` | 0.55s / 0.5s | 区块进入 / 遮罩 |
| `animate-pop-in` | popIn 0.38s | 弹窗面板 |
| `animate-word-rise` | wordRise 0.45s | 单词行 |

**所有进场动画时长 < 0.62s（视图过渡时长）** —— 这是"转场结束后不二次入场"的前提，见下。
`animate-word-rise` 只在首屏播放，翻页后抑制。

纸质书翻页（滑动，非 3D 翻书）：轨道 `w-[300%]` + 纸张 `basis-1/3`，`translateX((offset-1)*33.333%)`（offset 0 = 中间页居中）；拖拽时 `transition: none`，释放/翻页 `0.45s cubic-bezier(.22,.8,.22,1)`；翻页完成后轨道 key 自增重挂载（对齐复位，无过渡、无闪烁）。

### 常驻动画（环境层 `AmbientBackground.tsx`）

- 3 个色斑 `.ambient-orb-*`：**静态** `filter: blur(44px)`（桌面 72px），光栅化一次后作为合成层复用，不参与动画；
- 2 个有机轮廓环 `.ambient-halo-*`：`haloSpin` 78s / 104s 反向旋转（纯描边，无模糊）；
- 6 个漂浮字形 `.ambient-glyph`：`glyphDrift` 21–28s，位移/旋转/透明度由 CSS 变量 `--glyph-*` 驱动；
- 容器 `.ambient-stage`：`contain: layout paint style`，绘制与页面其余部分隔离；
- **不写常驻 `will-change`**：浏览器在动画期间自动提层、空闲自动降级；常驻会让 8 个层一直占显存；
- **移动端（<768px）`animation: none`**：视觉元素全部保留，只是不再播放，省电且零合成开销。

### 按压反馈与过渡范围

```css
:where(button:not(:disabled)) { transition: scale .14s cubic-bezier(.2,.8,.2,1), background-color .14s, color .14s }
:where(button:not(:disabled)):active { scale: .96 }
```
用独立 `scale` 属性而不是 `transform`：不覆盖组件自己的 hover 位移（Tailwind `--tw-translate-*`），也不产生跳动；`:where()` 零特异性，让组件自带的 `transition-*` 优先。

`[class*="transition-all"]` 统一收窄 `transition-property`（transform / opacity / 颜色 / 边框），**不再过渡 `box-shadow` / `filter`** —— 阴影逐帧重绘是滚动掉帧的主因。

### 视图过渡（View Transitions）

| 名字 | 挂载点 | 说明 |
|---|---|---|
| `app-nav-home` / `app-nav-book` | `.vt-nav-home`（HomeNav 的 fixed `<nav>`）/ `.vt-nav-book`（书页导航） | **两页各一个名字，不要合并**：内容完全不同，共用一个名字时 group 几何全程 identity、old/new 又是"立刻隐藏/立刻显示"→ 读起来是标题栏凭空消失又凭空出现。各自 `navBarOut` 退场 + `navBarIn` 落位，只在 `html.vt-nav-motion` 时生效（同页切 tab 不让整条栏闪） |
| `home-content` | `.vt-home` | 主页分区内容 |
| `book-page` | `.vt-book-page` | 书页（列表/纸质） |
| `detail-page` | `.vt-detail` | 全屏详情页 |
| ~~`detail-bar`~~ | 已删除 | 详情底栏**不要**挂名字：名字会把它从页面快照里单独摘出来、以 `translateY(100%)` 起步，进详情的一瞬间整条底栏不在画面里，随后"突然出现" |
| `dict-word-<slug>` + `view-transition-class: word` | `.vt-word` | 共享元素；**只有被点击的那个词挂名字**（`transitionNameFor`），否则几十个命名元素会拖垮转场 |

**防"发白 / 双重曝光"的总规则：同一时刻不能有两层都半透明。**
- 新快照 `opacity: 1` 恒定，只动 transform（下面永远压着旧快照，不会露白）；
- 旧快照淡出时，下面必须已经是完全不透明的新快照；
- 主页容器没有背景，旧首页淡到 0（0.16s）而不是留半透明，否则会和新内容叠成双重曝光；
- 共享词：旧快照 `opacity: 0`、新快照恒不透明并 `z-index: 1` 画在整屏详情页之上（否则被详情页盖住＝单词没动），飞行全程文字颜色恒定；`.vt-word { width: fit-content; max-width: 100% }` 让盒子贴合文字，避免"整行宽 586px → 标题 157px"的横向拉伸变形。

**不用 `animation: none` 抑制进场动画**：那会让动画在转场结束后从 0 补播（"闪一下"）。改为转场期间用 `opacity: 1 !important; transform: none !important` 压到终态，让时间线照常跑完（进场时长都 < 转场时长），撤销覆盖时无跳变。

**标记只活在过渡期间**：`withViewTransition` 在 start 前给 `<html>` 挂 `vt-running`（入场动画被压到 0.001s，等于立刻到终态），settle 时**先 `consumeEntranceAnimations()` 再摘类**。顺序不能反：浏览器会按"元素上仍声明着这条动画"重建一个 CSSAnimation，currentTime 按真实时间算 → 入场动画在转场后又播一遍（书架卡片、列表项最明显，`animation.finish()` 救不了，重建的是新对象）。`ENTRANCE_CLASSES` 必须与 `html.vt-running :is(...)` 名单逐项一致。不使用长效 `data-vt-entered` 标记——它会让转场过一次之后弹窗也失去入场动画。

所有入场动画时长必须 < 0.62s；stagger 延迟要封顶（列表 `Math.min(index, 6) * 55ms`），否则第 24 条要等 1.3s 才出现。

**不给 `::view-transition-group(*)` 设全局时长**：只给具体分组（home-content / detail-page / app-nav-home / app-nav-book / .word）单独定时，避免整页被一条长动画拖住。共享词的 group 用一段手写 `linear()` 缓动（0.85s），比默认 cubic-bezier 更"沉"、落点更稳。

`prefers-reduced-motion: reduce`：关闭全部 `animate-*`、环境层常驻动画、`transition-all`，以及全部 `::view-transition-old/new/group(*)` 动画（转场退化为瞬时切换）。

### 退场与状态（`[data-state]`）

**消失也必须动**。条件渲染（`{open && <X/>}`）在关闭时瞬间卸载，节点直接从 DOM 里消失，任何 transition 都来不及跑——用户看到的"退出动画完全没有"就是这么来的。

统一三件套：

| 件 | 位置 | 作用 |
|---|---|---|
| `usePresence(open, durationMs)` | `frontend/src/hooks/usePresence.ts` | 返回 `{ mounted, closing, entering }`：`closing` 让节点多活 durationMs 播退场；`entering` 只在"从关到开"的头两帧为 true（先以收起态画一帧，翻成展开态时 CSS 才有起点） |
| `[data-state]` + `.ui-pop` | `index.css` | `closing` = 淡出 + 下沉 6px（180ms）；组件只负责把状态写进 `data-state` |
| `.collapse` | `index.css` | `grid-template-rows: 0fr ↔ 1fr`，高度未知的内容（例句、附加说明）也能平滑展开/收起 |

写法：

```tsx
const p = usePresence(open, 220)
return p.mounted && (
  <div className="ui-pop" data-state={p.entering ? 'entering' : p.closing ? 'closing' : 'open'}>
```

已接入：词典栏（`.wb-rail[data-state]`，桌面收列宽 + 右移淡出、窄屏整张滑下）、设置/筛选/更多菜单、词典来源下拉、拼写建议（退场期间渲染上一帧的候选，别让动画作用在空面板上）、toast（2.4s 停留 + 0.2s 退场）、词典例句折叠、弹窗（`.modal-exit-overlay` / `.modal-exit`）。

**常驻挂载 vs 条件挂载**：有内部状态的浮层（词典栏的查询/勾选）必须常驻挂载，只用 `hidden` + `data-state` 控制显隐——条件挂载会在收起时把状态一起丢掉，导航按钮的 `aria-controls` 也会指向不存在的节点。退场期间给遮罩加 `pointer-events-none`，否则那 180ms 里它还在吃点击。

`prefers-reduced-motion` 下退场动画同样关闭（`closing` 直接不可见），不靠动画表达状态。

## 响应式

- 纸张/内容容器：`max-w-7xl mx-auto px-4 md:px-8`（详情页），随视口变化
- 纸质书列数：`grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5`
- 书架卡片（`Bookshelf.tsx`）：`flex flex-wrap justify-center gap-6`，单卡 `w-full max-w-sm`（移动端一行一张，桌面自动换行、居中对齐）；**加载态用同尺寸骨架卡**（封面 144px + 标题/描述行 + 操作区占位）预留高度，数据到位时首行不跳