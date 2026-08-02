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

/** 墨条遮挡：保留原文宽度，深炭墨条覆盖 */
function InkStripe({ text }: { text: string }) {
  return (
    <span className="rounded bg-charcoal text-transparent select-none shadow-[inset_0_2px_4px_rgba(0,0,0,0.3)]">
      {text}
    </span>
  )
}

const DRAG_THRESHOLD = 90

export default function PaperBookView({ data, page, onPrev, onNext, onAddFirst, fontScale }: Props) {
  // 遮挡状态：wordId → { word, def }，翻页即清空（翻开新一页）
  const [hidden, setHidden] = useState<Record<number, { word: boolean; def: boolean }>>({})
  // 翻页动画：out = 纸页翻走，in = 新页翻入
  const [flip, setFlip] = useState<'idle' | 'out' | 'in'>('idle')
  // 手势拖拽：当前横向位移
  const [dragDx, setDragDx] = useState(0)
  const timers = useRef<number[]>([])
  const startX = useRef(0)
  const dragging = useRef(false)
  const suppressClick = useRef(false)
  // 拖拽位移用 ref：快速拖拽时 move/up 可能同帧，避免读到旧 state
  const dragDxRef = useRef(0)

  useEffect(() => () => timers.current.forEach((t) => window.clearTimeout(t)), [])

  useEffect(() => {
    setHidden({})
  }, [page])

  // 新数据到达 → 新页从右侧翻入
  useEffect(() => {
    if (flip === 'out') {
      setFlip('in')
      timers.current.push(window.setTimeout(() => setFlip('idle'), 420))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data])

  function turnTo(dir: 'prev' | 'next') {
    if (flip !== 'idle' || !data) return
    if (dir === 'prev' && page <= 1) return
    if (dir === 'next' && page >= data.total_pages) return
    setFlip('out')
    timers.current.push(
      window.setTimeout(() => {
        if (dir === 'prev') onPrev()
        else onNext()
      }, 300),
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

  // —— 手势翻页：按住纸面拖动，释放时按位移翻页或回弹 ——
  function onPointerDown(e: React.PointerEvent) {
    if (flip !== 'idle' || !data) return
    if (e.pointerType === 'mouse' && e.button !== 0) return
    // 任意位置都可启动拖拽；按钮上无位移的点击仍走遮挡交互（click 不受影响）
    dragging.current = true
    startX.current = e.clientX
    dragDxRef.current = 0
    setDragDx(0)
  }

  function onPointerMove(e: React.PointerEvent) {
    if (!dragging.current) return
    const dx = e.clientX - startX.current
    // 边界限制：下一页方向（左拖）在末页禁用，上一页方向（右拖）在首页禁用
    if (data) {
      if (dx < 0 && page >= data.total_pages) return
      if (dx > 0 && page <= 1) return
    }
    dragDxRef.current = dx
    setDragDx(dx)
  }

  function onPointerUp() {
    if (!dragging.current) return
    dragging.current = false
    const dx = dragDxRef.current
    dragDxRef.current = 0
    setDragDx(0)
    // 有实际拖拽位移 → 抑制随后的 click（避免误触遮挡/热区）
    if (Math.abs(dx) > 8) suppressClick.current = true
    if (dx <= -DRAG_THRESHOLD) turnTo('next')
    else if (dx >= DRAG_THRESHOLD) turnTo('prev')
  }

  // —— 电子书式点击翻页：左 1/3 上一页，右 1/3 下一页 ——
  function onPaperClick(e: React.MouseEvent) {
    if (suppressClick.current) {
      suppressClick.current = false
      return
    }
    if ((e.target as HTMLElement).closest('button')) return
    if (flip !== 'idle' || !data) return
    const rect = e.currentTarget.getBoundingClientRect()
    const x = e.clientX - rect.left
    if (x < rect.width / 3) turnTo('prev')
    else if (x > (rect.width * 2) / 3) turnTo('next')
  }

  if (!data) return <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>

  const words = data.items

  if (words.length === 0) {
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

  function toggleWord(id: number) {
    setHidden((h) => ({ ...h, [id]: { word: !h[id]?.word, def: h[id]?.def ?? false } }))
  }

  function toggleDef(id: number) {
    setHidden((h) => ({ ...h, [id]: { word: h[id]?.word ?? false, def: !h[id]?.def } }))
  }

  // 纸张位移：拖拽时跟随指针，否则走翻页动画
  const transform = dragging.current
    ? `translateX(${dragDx}px) rotateY(${dragDx * 0.08}deg)`
    : flip === 'out'
      ? 'translateX(-16%) rotateY(8deg)'
      : 'translateX(0) rotateY(0)'
  const paperTransition = dragging.current
    ? 'none'
    : 'transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1)'
  const paperOpacity = dragging.current
    ? Math.min(1, 1 - Math.abs(dragDx) / 1400)
    : flip === 'out'
      ? 0.45
      : 1
  const progress = data.total_pages > 1 ? Math.round((page / data.total_pages) * 100) : 100
  const f = fontStyles(fontScale)

  return (
    <div className="[perspective:1600px]">
      {/* 纸堆：下层露出一张纸，上层可拖拽/点击翻页 */}
      <div className="relative w-full group">
        {/* 下层纸 */}
        <div
          aria-hidden="true"
          className="absolute inset-0 rounded-2xl bg-[#F2EAE0] shadow-sm border border-charcoal/5 translate-x-2 translate-y-2"
        />
        {/* 上层纸 */}
        <div
          className={`relative w-full bg-[#FDFAF4] rounded-2xl shadow-sm border border-charcoal/5 overflow-hidden select-none cursor-pointer ${flip === 'in' ? 'animate-page-in' : ''}`}
          style={{
            touchAction: 'pan-y',
            transform,
            transition: paperTransition,
            opacity: paperOpacity,
          }}
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
            className="absolute inset-y-0 left-0 w-1/3 flex items-center pl-3 pointer-events-none opacity-0 md:group-hover:opacity-30 transition-opacity"
          >
            <ChevronLeftIcon className="w-7 h-7 text-charcoal/70" />
          </div>
          <div
            aria-hidden="true"
            className="absolute inset-y-0 right-0 w-1/3 flex items-center justify-end pr-3 pointer-events-none opacity-0 md:group-hover:opacity-30 transition-opacity"
          >
            <ChevronRightIcon className="w-7 h-7 text-charcoal/70" />
          </div>

          {/* 左侧装订线：靠左，内容区整体右移避开 */}
          <div
            aria-hidden="true"
            className="absolute left-5 top-0 bottom-0 w-px bg-[#E9B8BC]/70"
          />
          {/* 页眉 */}
          <div className="relative px-10 md:px-12 pt-5 pb-2 flex items-center justify-center text-[11px] tracking-[0.2em] uppercase text-charcoal/35">
            <span>Page {page}</span>
          </div>

          {/* 多列单词：一行横线上多个单词，单词在线上方、释义在线下方 */}
          <div className="relative grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 2xl:grid-cols-5 px-9 md:px-11">
            {words.map((w) => {
              const h = hidden[w.id] ?? { word: false, def: false }
              const full = formatDefinitions(w)
              return (
                <div key={w.id} className="group/cell px-2 md:px-3 hover:bg-sand/15 transition-colors">
                  {/* 单词（横线上方，底部贴线） */}
                  <div className="flex items-end gap-2 min-w-0" style={{ height: f.rowH }}>
                    <button
                      onClick={() => toggleWord(w.id)}
                      aria-pressed={h.word}
                      title={h.word ? '点击显示单词' : '点击遮挡单词，回忆拼写'}
                      style={f.word}
                      className="font-serif text-charcoal tracking-wide leading-none pb-[3px] truncate rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
                    >
                      {h.word ? <InkStripe text={w.spelling} /> : w.spelling}
                    </button>
                    {w.phonetic && (
                      <span
                        style={f.phonetic}
                        className="shrink-0 text-charcoal/40 tracking-wide leading-none pb-[4px] truncate"
                      >
                        {w.phonetic}
                      </span>
                    )}
                  </div>
                  {/* 横线 */}
                  <div aria-hidden="true" className="h-px bg-[#E7DAC6]" />
                  {/* 释义（横线下方）：可换行显示，最多 3 行，不再压缩成一行 */}
                  <div className="pt-[3px] pb-1 min-w-0" style={{ minHeight: f.defMinH }}>
                    <button
                      onClick={() => toggleDef(w.id)}
                      aria-pressed={h.def}
                      title={h.def ? '点击显示释义' : full}
                      style={f.def}
                      className="w-full text-charcoal/60 leading-snug line-clamp-3 text-left rounded -mx-1 px-1 transition-colors duration-150 hover:bg-sand/50 focus-visible:outline-2 focus-visible:outline-clay focus-visible:outline-offset-4"
                    >
                      {h.def ? <InkStripe text={full} /> : full}
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
                style={{ width: `${progress}%` }}
              />
            </div>
            <p className="mt-3 text-center text-[11px] tracking-[0.2em] uppercase text-charcoal/30">
              第 {page} 页 · 共 {data.total_pages} 页
            </p>
          </div>
        </div>
      </div>
    </div>
  )
}
