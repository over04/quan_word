import { Fragment, memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { type Page, type Tag, type Word } from '../api'
import { BookIcon, PlusIcon, WrenchIcon } from './Icons'
import TagQuickModal from './TagQuickModal'

interface Props {
  /** 已加载页，按页码升序且连续；d 必非空（未就绪页不渲染） */
  pages: Array<{ d: Page<Word>; pageNo: number }>
  /** 正在加载下一页（底部显示"加载中…"占位） */
  loading: boolean
  /** 哨兵进入视口（提前 800px）时触发；参数 = 要加载的下一页页码与该书总页数 */
  onReachEnd: (nextPageNo: number, totalPages: number) => void
  onAddFirst: () => void
  /** 单词字号（px，12–28），音标/释义/行高按比例联动 */
  fontScale: number
  /** 基线：整书单词（含音标）/ 释义 是否全隐藏 */
  hideAllWord: boolean
  hideAllDef: boolean
  /** 手动点过的词：wordId → 该词绝对隐藏状态（例外，持久化，父级持有） */
  wordDiff: Record<number, boolean>
  defDiff: Record<number, boolean>
  /** 手动点击单词/释义：父级按当前实际显示翻转并持久化 */
  onToggleWord: (id: number) => void
  onToggleDef: (id: number) => void
  /** 该书全部标签（词块 chips 与快速弹窗用） */
  tags: Tag[]
  /** 单词标签集变更成功后回调（父级刷新数据） */
  onTagsUpdated: () => void
  /** 新建标签成功后回调（父级刷新标签列表） */
  onTagsCreated: (tag: Tag) => void
}

/** 按字号计算出的各元素样式（fontStyles 的返回契约） */
export interface FontStyles {
  word: { fontSize: string }
  phonetic: { fontSize: string }
  def: { fontSize: string }
  rowH: number
  defMinH: number
  /** 标签 chip 字号 px（随字号联动，最小 10px） */
  chipFont: number
  /** 行内图标尺寸 px（随字号联动，最小 10px） */
  iconSize: number
}

/** 按字号数字计算各元素尺寸：字号 px → 各元素样式 */
function fontStyles(fontScale: number): FontStyles {
  return {
    word: { fontSize: `${fontScale}px` },
    phonetic: { fontSize: `${Math.round(fontScale * 0.55)}px` },
    def: { fontSize: `${Math.round(fontScale * 0.6)}px` },
    rowH: fontScale * 1.7,
    defMinH: fontScale * 1.4,
    chipFont: Math.max(10, Math.round(fontScale * 0.5)),
    iconSize: Math.max(10, Math.round(fontScale * 0.6)),
  }
}

/** 释义文本：词性 + 内容 */
function formatDefinitions(w: Word): string {
  return w.definitions.map((d) => (d.pos ? `${d.pos} ${d.meaning}` : d.meaning)).join('；')
}

/** 遮挡 = 模糊：中心浓雾向四周宽范围渐渐变淡（单层径向 mask，椭圆放大 + 62% 过渡带），融入纸面无切割感 */
function Covered({ text }: { text: string }) {
  return (
    <span
      className="inline-block max-w-full select-none break-words blur-[6px] opacity-65 transition-[filter,opacity] duration-300"
      style={{
        WebkitMaskImage:
          'radial-gradient(ellipse 150% 160% at 50% 50%, black 12%, transparent 74%)',
        maskImage: 'radial-gradient(ellipse 150% 160% at 50% 50%, black 12%, transparent 74%)',
        // 整页遮挡时大量 blur 元素：提示合成器独立图层，避免频繁重栅格化
        willChange: 'filter',
      }}
    >
      {text}
    </span>
  )
}

/** 响应式列数：与 Tailwind 断点一致（grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5） */
function colCount() {
  const w = window.innerWidth
  if (w >= 1536) return 5
  if (w >= 1024) return 4
  if (w >= 640) return 3
  return 2
}

/** 单词区 props（供 memo 组件使用；引用稳定的回调由父级 useCallback 保证） */
interface WordLineCellProps {
  w: Word
  /** 是否播放进入动画（仅首屏） */
  anim: boolean
  /** 词条在已加载单词流中的序号（动画错峰用） */
  index: number
  f: FontStyles
  hideAllWord: boolean
  wordDiff: Record<number, boolean>
  onToggleWord: (id: number) => void
}

/** 单词区：横线上方的单词 + 音标（长词折行撑高，不截断不溢出） */
const WordLineCell = memo(function WordLineCell({
  w,
  anim,
  index,
  f,
  hideAllWord,
  wordDiff,
  onToggleWord,
}: WordLineCellProps) {
  // 基线 + 手动例外：手动点过的词用其绝对状态，其余跟随基线（无优先级，最后操作者生效）
  const wordHidden = w.id in wordDiff ? wordDiff[w.id] : hideAllWord
  return (
    <div
      className={`group/cell px-2 md:px-3 pt-1 min-w-0 hover:bg-sand/15 transition-colors ${anim ? 'animate-word-rise' : ''}`}
      style={anim ? { animationDelay: `${Math.min(index, 8) * 40}ms` } : undefined}
    >
      <div className="flex items-end gap-2 min-w-0" style={{ minHeight: f.rowH }}>
        <button
          onClick={() => onToggleWord(w.id)}
          aria-pressed={wordHidden}
          title={wordHidden ? '点击显示单词' : '点击遮挡单词，回忆拼写'}
          style={f.word}
          className="min-w-0 font-serif text-charcoal tracking-wide leading-tight pb-[3px] break-words rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
        >
          {wordHidden ? <Covered text={w.spelling} /> : w.spelling}
        </button>
        {w.phonetic && (
          <span
            style={f.phonetic}
            className="min-w-0 text-charcoal/40 tracking-wide leading-tight pb-[4px] break-words"
          >
            {wordHidden ? <Covered text={w.phonetic} /> : w.phonetic}
          </span>
        )}
      </div>
    </div>
  )
})

/** 释义 + 标签区 props */
interface WordDefCellProps {
  w: Word
  anim: boolean
  index: number
  f: FontStyles
  hideAllDef: boolean
  defDiff: Record<number, boolean>
  onToggleDef: (id: number) => void
  tagName: Map<number, string>
  onOpenQuick: (w: Word) => void
}

/** 释义 + 标签区：横线下方的释义（完整多行）与标签 chips + 添加按钮 */
const WordDefCell = memo(function WordDefCell({
  w,
  anim,
  index,
  f,
  hideAllDef,
  defDiff,
  onToggleDef,
  tagName,
  onOpenQuick,
}: WordDefCellProps) {
  const defHidden = w.id in defDiff ? defDiff[w.id] : hideAllDef
  const full = formatDefinitions(w)
  return (
    <div
      className={`group/cell px-2 md:px-3 pt-[3px] pb-1 min-w-0 hover:bg-sand/15 transition-colors ${anim ? 'animate-word-rise' : ''}`}
      style={anim ? { animationDelay: `${Math.min(index, 8) * 40}ms` } : undefined}
    >
      {/* 释义：完整多行显示，不截断 */}
      <div className="pb-1 min-w-0" style={{ minHeight: f.defMinH }}>
        <button
          onClick={() => onToggleDef(w.id)}
          aria-pressed={defHidden}
          title={defHidden ? '点击显示释义' : full}
          style={f.def}
          className="w-full text-charcoal/60 leading-snug text-left rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
        >
          {defHidden ? <Covered text={full} /> : full}
        </button>
      </div>
      {/* 标签行：已有标签 chips（纯展示，字号随字号联动）+ 扳手按钮（管理标签，增删都在面板内） */}
      <div className="mt-1 flex items-center gap-1 min-w-0">
        <div className="flex items-center gap-1 min-w-0 overflow-hidden">
          {w.tags.map((tid) => (
            <span
              key={tid}
              style={{ fontSize: f.chipFont }}
              className="shrink-0 inline-flex items-center px-1.5 py-0.5 rounded font-medium bg-sage/50 text-charcoal/70"
            >
              {tagName.get(tid) ?? tid}
            </span>
          ))}
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation()
            onOpenQuick(w)
          }}
          title="管理标签"
          aria-label={`管理 ${w.spelling} 的标签`}
          style={{ width: f.iconSize + 6, height: f.iconSize + 6 }}
          className="shrink-0 rounded-full flex items-center justify-center text-charcoal/25 hover:text-clay hover:bg-sand/50 transition-colors"
        >
          <WrenchIcon className="" style={{ width: f.iconSize, height: f.iconSize }} />
        </button>
      </div>
    </div>
  )
})

export default function PaperBookView({
  pages,
  loading,
  onReachEnd,
  onAddFirst,
  fontScale,
  hideAllWord,
  hideAllDef,
  wordDiff,
  defDiff,
  onToggleWord,
  onToggleDef,
  tags: allTags,
  onTagsUpdated,
  onTagsCreated,
}: Props) {
  // 快速标签编辑：当前目标单词（null = 未打开）
  const [quickWord, setQuickWord] = useState<Word | null>(null)
  // 标签 id → 名称映射（词块 chips）；引用稳定，避免低频状态变化时重渲染
  const tagName = useMemo(() => new Map(allTags.map((t) => [t.id, t.name])), [allTags])
  // 单词行进入动画：仅首屏播放；滚动追加的页不播（避免滚动时动画干扰）
  const [wordsAnim, setWordsAnim] = useState(true)
  useEffect(() => {
    if (pages.length > 1) setWordsAnim(false)
  }, [pages.length])

  // 响应式列数（resize 时重排切片）；f 按字号缓存（保持 memo 词块 props 稳定）
  const [cols, setCols] = useState(colCount)
  useEffect(() => {
    const onResize = () => setCols(colCount())
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])
  const f = useMemo(() => fontStyles(fontScale), [fontScale])
  // 已加载词条扁平化（跨页连续流）
  const allWords = useMemo(() => pages.flatMap(({ d }) => d.items), [pages])

  // 哨兵：observer 随 pages 长度/加载状态重建——IO 仅在相交状态变化时回调，内容不足视口时哨兵保持相交、只触发一次，
  // 追加新页或加载结束后重建 observer 会立即重新评估相交，递归填充直到哨兵被推出视口（或到达末页）；
  // 回调通过 refs 读最新 pages/onReachEnd
  const pagesRef = useRef(pages)
  useEffect(() => {
    pagesRef.current = pages
  }, [pages])
  const onReachEndRef = useRef(onReachEnd)
  useEffect(() => {
    onReachEndRef.current = onReachEnd
  }, [onReachEnd])
  const sentinelRef = useRef<HTMLDivElement | null>(null)
  const hasPages = pages.length > 0
  // 上次触发尝试（时间 + 当时页数）：加载失败/进行中时 observer 重建会立即回调，但"无新页且 1.5s 内"跳过，
  // 避免失败重试风暴；用户滚动（相交状态变化）或间隔后自然重试
  const lastAttemptRef = useRef({ at: 0, len: 0 })
  useEffect(() => {
    if (!hasPages) return
    const el = sentinelRef.current
    if (!el) return
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries.some((e) => e.isIntersecting)) return
        const len = pagesRef.current.length
        if (len === 0) return
        const t = Date.now()
        const last = lastAttemptRef.current
        if (len === last.len && t - last.at < 1500) return
        lastAttemptRef.current = { at: t, len }
        const p = pagesRef.current[len - 1]
        onReachEndRef.current(p.pageNo + 1, p.d.total_pages)
      },
      { rootMargin: '0px 0px 800px 0px' },
    )
    io.observe(el)
    return () => io.disconnect()
  }, [hasPages, pages.length, loading])

  /** 打开单词标签管理（增删都在面板内完成） */
  const openQuick = useCallback((w: Word) => {
    setQuickWord(w)
  }, [])

  /** 标签变更后：抑制词行重播动画，再通知父级刷新 */
  function handleTagsUpdated() {
    setWordsAnim(false)
    onTagsUpdated()
  }

  if (pages.length === 0) {
    return <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>
  }

  if (pages[0].d.items.length === 0) {
    return (
      <div className="max-w-md mx-auto text-center py-16 animate-fade-in-up">
        <div className="mx-auto w-20 h-20 rounded-2xl bg-sand/40 flex items-center justify-center text-clay">
          <BookIcon className="w-9 h-9" />
        </div>
        <h2 className="font-serif text-3xl text-charcoal mt-7">这本书还没有单词</h2>
        <p className="mt-3 text-charcoal/60 leading-relaxed">添加第一个单词，开始第一页。</p>
        <button
          onClick={onAddFirst}
          className="mt-8 bg-charcoal text-ivory px-8 py-3.5 rounded-full font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 inline-flex items-center gap-2"
        >
          <PlusIcon className="w-4 h-4" />
          添加单词
        </button>
      </div>
    )
  }

  return (
    <div className="relative">
      {/* 左侧装订线：贯穿整个单词流（数据按页懒加载，视觉连续） */}
      <div aria-hidden="true" className="absolute left-5 top-0 bottom-0 w-px bg-[#E9B8BC]/70" />

      <div className="relative px-9 md:px-11 pt-6">
        {/* 连续网格：词条按"行组"切片（每行 cols 个）——先全部单词区填满一行 → 横线行（同行天然对齐）→ 释义行 → 下一行组；页边界不断行 */}
        <div
          className="grid content-start"
          style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
        >
          {Array.from({ length: Math.ceil(allWords.length / cols) }, (_, ci) => {
            const chunk = allWords.slice(ci * cols, ci * cols + cols)
            // 末行组不满 cols 时补空占位：grid 流式填充下若行 1 填不满，横线会串入单词行导致列错位
            // 三个区各自独立占位（同父级 key 必须唯一）
            const pads = (tag: string) =>
              Array.from({ length: cols - chunk.length }, (_, pi) => (
                <span key={`${tag}${ci}-${pi}`} aria-hidden="true" className="min-w-0" />
              ))
            return (
              <Fragment key={ci}>
                {chunk.map((w, i) => (
                  <WordLineCell
                    key={`w${w.id}`}
                    w={w}
                    anim={wordsAnim}
                    index={ci * cols + i}
                    f={f}
                    hideAllWord={hideAllWord}
                    wordDiff={wordDiff}
                    onToggleWord={onToggleWord}
                  />
                ))}
                {pads('w')}
                {/* 横线：独占一行，同一行组内所有横线同一条水平线 */}
                {chunk.map((w) => (
                  <div key={`l${w.id}`} aria-hidden="true" className="h-px bg-[#E7DAC6]" />
                ))}
                {pads('l')}
                {chunk.map((w, i) => (
                  <WordDefCell
                    key={`d${w.id}`}
                    w={w}
                    anim={wordsAnim}
                    index={ci * cols + i}
                    f={f}
                    hideAllDef={hideAllDef}
                    defDiff={defDiff}
                    onToggleDef={onToggleDef}
                    tagName={tagName}
                    onOpenQuick={openQuick}
                  />
                ))}
                {pads('d')}
              </Fragment>
            )
          })}
        </div>
      </div>

      {loading && <p className="text-center text-charcoal/40 py-8 animate-pulse">加载中…</p>}
      <div ref={sentinelRef} aria-hidden="true" className="h-px" />

      {quickWord && (
        <TagQuickModal
          bookId={quickWord.wordbook_id}
          word={quickWord}
          tags={allTags}
          onClose={() => setQuickWord(null)}
          onChanged={handleTagsUpdated}
          onTagsCreated={onTagsCreated}
        />
      )}
    </div>
  )
}
