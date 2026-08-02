import { useEffect, useRef, useState } from 'react'
import type { Page, Word } from '../api'
import { BookIcon, ChevronLeftIcon, ChevronRightIcon, PlusIcon } from './Icons'

interface Props {
  data: Page<Word> | null
  page: number
  onPrev: () => void
  onNext: () => void
  onAddFirst: () => void
  /** 单词字号（px，12–28），音标/释义/行高按比例联动 */
  fontScale: number
  /** 一键模糊：整页单词（含音标）/ 释义 */
  coverWord: boolean
  coverDef: boolean
  /** 相邻页数据（预取缓存），滑动翻页过程中可见 */
  prevPage: Page<Word> | null
  nextPage: Page<Word> | null
}

/** 按字号数字计算各元素尺寸：字号 px → 各元素样式 */
function fontStyles(fontScale: number) {
  return {
    word: { fontSize: `${fontScale}px` },
    phonetic: { fontSize: `${Math.round(fontScale * 0.55)}px` },
    def: { fontSize: `${Math.round(fontScale * 0.6)}px` },
    rowH: fontScale * 1.7,
    defMinH: fontScale * 1.4,
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
      className="inline-block select-none blur-[6px] opacity-65 transition-[filter,opacity] duration-300"
      style={{
        WebkitMaskImage:
          'radial-gradient(ellipse 150% 160% at 50% 50%, black 12%, transparent 74%)',
        maskImage: 'radial-gradient(ellipse 150% 160% at 50% 50%, black 12%, transparent 74%)',
      }}
    >
      {text}
    </span>
  )
}

const DRAG_THRESHOLD_PX = 90
const SLIDE_MS = 450

interface SheetProps {
  d: Page<Word> | null
  pageNo: number
  fontScale: number
  coverWord: boolean
  coverDef: boolean
  hidden: Record<number, { word: boolean; def: boolean }>
  onToggleWord: (id: number) => void
  onToggleDef: (id: number) => void
  /** 单词行进入动画：仅首次加载播放 */
  wordsAnim: boolean
}

/** 一张纸：页眉 + 横线单词网格 + 页脚；数据未就绪时渲染空白纸 */
function Sheet({
  d,
  pageNo,
  fontScale,
  coverWord,
  coverDef,
  hidden,
  onToggleWord,
  onToggleDef,
  wordsAnim,
}: SheetProps) {
  const f = fontStyles(fontScale)
  const words = d?.items ?? []
  const pct = d && d.total_pages > 1 ? Math.round((pageNo / d.total_pages) * 100) : 100

  return (
    <div className="relative w-full shrink-0 basis-1/3 min-h-[calc(100dvh-12rem)] bg-[#FDFAF4] select-none flex flex-col">
      {/* 左侧装订线：靠左，内容区整体右移避开 */}
      <div aria-hidden="true" className="absolute left-5 top-0 bottom-0 w-px bg-[#E9B8BC]/70" />
      {/* 页眉 */}
      <div className="relative px-10 md:px-12 pt-5 pb-2 flex items-center justify-center text-[11px] tracking-[0.2em] uppercase text-charcoal/35">
        <span>Page {pageNo}</span>
      </div>

      {/* 多列单词：一行横线上多个单词，单词在线上方、释义在线下方 */}
      <div className="relative grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5 flex-1 content-start px-9 md:px-11">
        {words.map((w, i) => {
          const h = hidden[w.id] ?? { word: false, def: false }
          const full = formatDefinitions(w)
          return (
            <div
              key={w.id}
              className={`group/cell px-2 md:px-3 hover:bg-sand/15 transition-colors ${wordsAnim ? 'animate-word-rise' : ''}`}
              style={wordsAnim ? { animationDelay: `${Math.min(i, 8) * 40}ms` } : undefined}
            >
              {/* 单词（横线上方，底部贴线）+ 音标：一键模糊时单词与音标一起晕开 */}
              <div className="flex items-end gap-2 min-w-0" style={{ height: f.rowH }}>
                <button
                  onClick={() => onToggleWord(w.id)}
                  aria-pressed={h.word}
                  title={h.word ? '点击显示单词' : '点击遮挡单词，回忆拼写'}
                  style={f.word}
                  className="font-serif text-charcoal tracking-wide leading-none pb-[3px] truncate rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
                >
                  {h.word || coverWord ? <Covered text={w.spelling} /> : w.spelling}
                </button>
                {w.phonetic && (
                  <span
                    style={f.phonetic}
                    className="shrink-0 text-charcoal/40 tracking-wide leading-none pb-[4px] truncate"
                  >
                    {coverWord ? <Covered text={w.phonetic} /> : w.phonetic}
                  </span>
                )}
              </div>
              {/* 横线 */}
              <div
                aria-hidden="true"
                className="h-px bg-[#E7DAC6] transition-colors duration-300 group-hover/cell:bg-clay/50"
              />
              {/* 释义（横线下方）：可换行显示，最多 3 行，不再压缩成一行 */}
              <div className="pt-[3px] pb-1 min-w-0" style={{ minHeight: f.defMinH }}>
                <button
                  onClick={() => onToggleDef(w.id)}
                  aria-pressed={h.def}
                  title={h.def ? '点击显示释义' : full}
                  style={f.def}
                  className="w-full text-charcoal/60 leading-snug line-clamp-3 text-left rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
                >
                  {h.def || coverDef ? <Covered text={full} /> : full}
                </button>
              </div>
            </div>
          )
        })}
      </div>

      {/* 页脚：进度条 + 页码 */}
      <div className="relative px-10 md:px-12 pt-6 pb-5">
        <div className="h-0.5 bg-charcoal/10 rounded-full overflow-hidden">
          <div
            className="h-full bg-clay rounded-full transition-all duration-300"
            style={{ width: `${pct}%` }}
          />
        </div>
        <p className="mt-3 text-center text-[11px] tracking-[0.2em] uppercase text-charcoal/30">
          第 {pageNo} 页 · 共 {d?.total_pages ?? 0} 页
        </p>
      </div>
    </div>
  )
}

export default function PaperBookView({
  data,
  page,
  onPrev,
  onNext,
  onAddFirst,
  fontScale,
  coverWord,
  coverDef,
  prevPage,
  nextPage,
}: Props) {
  // 遮挡状态：wordId → { word, def }，翻页即清空（翻开新一页）
  const [hidden, setHidden] = useState<Record<number, { word: boolean; def: boolean }>>({})
  // 滑动翻页：offset 单位 = 页宽（0 = 当前页；-1 = 右侧相邻页可见；+1 = 左侧相邻页可见）
  const [offset, setOffset] = useState(0)
  const [sliding, setSliding] = useState(false)
  const [dragging, setDragging] = useState(false)
  // 轨道 key：翻页完成时重挂载轨道，保证复位位（中间页对齐）绝无过渡动画
  const [trackKey, setTrackKey] = useState(0)
  // 单词行进入动画：仅首次加载播放；翻页后抑制（重挂载不再触发淡入）
  const [wordsAnim, setWordsAnim] = useState(true)
  // 三页轨道：[上一页, 当前页, 下一页]，拖拽中相邻页直接可见
  const [triple, setTriple] = useState<{
    left: { d: Page<Word> | null; pageNo: number }
    mid: { d: Page<Word> | null; pageNo: number }
    right: { d: Page<Word> | null; pageNo: number }
  } | null>(null)
  const draggingRef = useRef(false)
  const startX = useRef(0)
  const dxRef = useRef(0)
  const widthRef = useRef(1)
  const suppressClick = useRef(false)
  const animDirRef = useRef<'next' | 'prev' | null>(null)
  const timers = useRef<number[]>([])
  // 滑动动画是否已结束（复位必须等动画结束，否则 cache 命中时动画被截断成闪烁）
  const animEndedRef = useRef(true)
  const pendingResetRef = useRef(false)

  useEffect(() => () => timers.current.forEach((t) => window.clearTimeout(t)), [])

  // 遮挡随翻页清空（翻开新一页）
  useEffect(() => {
    setHidden({})
  }, [page])

  /** 数据就位复位：重排三页轨道，重挂载轨道回到对齐位（无过渡） */
  function doReset() {
    animDirRef.current = null
    setSliding(false)
    setOffset(0)
    setTriple({
      left: { d: prevPage, pageNo: page - 1 },
      mid: { d: data, pageNo: page },
      right: { d: nextPage, pageNo: page + 1 },
    })
    setTrackKey((k) => k + 1)
  }

  // 当前页数据到达：动画已结束才复位（数据是动画结束后才加载的，通常立即可复位）
  useEffect(() => {
    if (!animEndedRef.current) {
      pendingResetRef.current = true
      return
    }
    doReset()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data])

  // 相邻页数据到达（预取完成）→ 填充对应纸位；翻页动画中更新无碍（动画结束会整体重置）
  useEffect(() => {
    setTriple((t) => (t ? { ...t, left: { d: prevPage, pageNo: page - 1 } } : t))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [prevPage])

  useEffect(() => {
    setTriple((t) => (t ? { ...t, right: { d: nextPage, pageNo: page + 1 } } : t))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nextPage])

  function turnTo(dir: 'next' | 'prev') {
    if (sliding || !data) return
    if (dir === 'prev' && page <= 1) return
    if (dir === 'next' && page >= data.total_pages) return
    animDirRef.current = dir
    animEndedRef.current = false
    setWordsAnim(false)
    setSliding(true)
    setOffset(dir === 'next' ? -1 : 1)
    timers.current.push(
      window.setTimeout(() => {
        animEndedRef.current = true
        if (dir === 'next') onNext()
        else onPrev()
        if (pendingResetRef.current) {
          pendingResetRef.current = false
          doReset()
        }
      }, SLIDE_MS),
    )
  }

  // —— 键盘翻页（← / →）——
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const t = e.target as HTMLElement
      if (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable) return
      if (e.key === 'ArrowRight') turnTo('next')
      if (e.key === 'ArrowLeft') turnTo('prev')
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })

  // —— 手势滑动：拖动时双页跟手平移，释放后按位移翻页或回弹 ——
  function onPointerDown(e: React.PointerEvent) {
    if (sliding || !data) return
    if (e.pointerType === 'mouse' && e.button !== 0) return
    draggingRef.current = true
    startX.current = e.clientX
    dxRef.current = 0
    widthRef.current = e.currentTarget.getBoundingClientRect().width || 1
    setDragging(true)
  }

  function onPointerMove(e: React.PointerEvent) {
    if (!draggingRef.current) return
    const dx = e.clientX - startX.current
    // 边界限制：下一页方向（左拖）在末页禁用，上一页方向（右拖）在首页禁用
    if (data) {
      if (dx < 0 && page >= data.total_pages) return
      if (dx > 0 && page <= 1) return
    }
    dxRef.current = dx
    setOffset(Math.max(-1, Math.min(1, dx / widthRef.current)))
  }

  function onPointerUp() {
    if (!draggingRef.current) return
    draggingRef.current = false
    setDragging(false)
    const dx = dxRef.current
    dxRef.current = 0
    // 有实际拖拽位移 → 抑制随后的 click（避免误触遮挡/热区）
    if (Math.abs(dx) > 8) suppressClick.current = true
    const far = Math.abs(dx) / widthRef.current
    if (dx <= -DRAG_THRESHOLD_PX || (dx < 0 && far > 0.18)) turnTo('next')
    else if (dx >= DRAG_THRESHOLD_PX || (dx > 0 && far > 0.18)) turnTo('prev')
    else setOffset(0) // 未过阈值：回弹
  }

  // —— 电子书式点击翻页：左 1/3 上一页，右 1/3 下一页 ——
  function onPaperClick(e: React.MouseEvent) {
    if (suppressClick.current) {
      suppressClick.current = false
      return
    }
    if ((e.target as HTMLElement).closest('button')) return
    if (sliding || !data) return
    const rect = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - rect.left
    if (x < rect.width / 3) turnTo('prev')
    else if (x > (rect.width * 2) / 3) turnTo('next')
  }

  function toggleWord(id: number) {
    setHidden((h) => ({ ...h, [id]: { word: !h[id]?.word, def: h[id]?.def ?? false } }))
  }

  function toggleDef(id: number) {
    setHidden((h) => ({ ...h, [id]: { word: h[id]?.word ?? false, def: !h[id]?.def } }))
  }

  if (!data) return <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>

  if (data.items.length === 0) {
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
    <div>
      {/* 纸堆：下层露出一张纸（书芯），上层可滑动/点击翻页 */}
      <div className="relative w-full group">
        {/* 下层纸 */}
        <div
          aria-hidden="true"
          className="absolute inset-0 rounded-2xl bg-[#F2EAE0] shadow-sm border border-charcoal/5 translate-x-2 translate-y-2"
        />
        {/* 舞台：裁切滑动区 */}
        <div
          className="relative overflow-hidden rounded-2xl bg-[#FDFAF4] border border-charcoal/5 shadow-[0_8px_24px_rgb(47_42_37/0.06)] cursor-pointer"
          style={{ touchAction: 'pan-y' }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerLeave={onPointerUp}
          onPointerCancel={onPointerUp}
          onClick={onPaperClick}
        >
          {/* 左右翻页热区提示（桌面 hover 显示） */}
          <div
            aria-hidden="true"
            className="absolute inset-y-0 left-0 w-1/3 flex items-center pl-3 pointer-events-none opacity-0 md:group-hover:opacity-30 transition-opacity z-10"
          >
            <ChevronLeftIcon className="w-7 h-7 text-charcoal/70" />
          </div>
          <div
            aria-hidden="true"
            className="absolute inset-y-0 right-0 w-1/3 flex items-center justify-end pr-3 pointer-events-none opacity-0 md:group-hover:opacity-30 transition-opacity z-10"
          >
            <ChevronRightIcon className="w-7 h-7 text-charcoal/70" />
          </div>

          {/* 三页轨道：当前页居中，相邻页在两侧（滑动时直接可见） */}
          {/* 三页轨道：当前页居中，相邻页在两侧（滑动时直接可见）；key 变化 = 翻页完成重挂载对齐 */}
          <div
            key={trackKey}
            className="flex w-[300%]"
            style={{
              // 轨道 = 3 页宽（w-[300%]）：offset 0 = 中间页居中（平移 1 页 = 轨道 1/3），±1 = 相邻页居中
              transform: `translateX(${(offset - 1) * 33.333333}%)`,
              transition: dragging
                ? 'none'
                : 'transform 0.45s cubic-bezier(0.22, 0.8, 0.22, 1)',
            }}
          >
            {triple && (
              <>
                <Sheet
                  d={triple.left.d}
                  pageNo={triple.left.pageNo}
                  fontScale={fontScale}
                  coverWord={coverWord}
                  coverDef={coverDef}
                  hidden={hidden}
                  onToggleWord={toggleWord}
                  onToggleDef={toggleDef}
                  wordsAnim={wordsAnim}
                />
                <Sheet
                  d={triple.mid.d}
                  pageNo={triple.mid.pageNo}
                  fontScale={fontScale}
                  coverWord={coverWord}
                  coverDef={coverDef}
                  hidden={hidden}
                  onToggleWord={toggleWord}
                  onToggleDef={toggleDef}
                  wordsAnim={wordsAnim}
                />
                <Sheet
                  d={triple.right.d}
                  pageNo={triple.right.pageNo}
                  fontScale={fontScale}
                  coverWord={coverWord}
                  coverDef={coverDef}
                  hidden={hidden}
                  onToggleWord={toggleWord}
                  onToggleDef={toggleDef}
                  wordsAnim={wordsAnim}
                />
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
