import { useEffect, useRef, useState } from 'react'
import type { DictSuggestion, WordPresenceBook } from '../api'
import { transitionNameFor } from '../viewTransition'
import { ArrowRightIcon, RefreshIcon, SearchIcon } from './Icons'

interface Props {
  /** 当前输入词（空态/错误态里直接查这个词） */
  query: string
  suggestions: DictSuggestion[]
  /** 小写拼写 → 已加入的词书 */
  presence: Record<string, WordPresenceBook[]>
  loading: boolean
  error: string
  onSelect: (word: string) => void
  onRetry: () => void
}

/** 首屏先给 6 条，之后每次多放 6 条（后端一次给 24 条，够翻三轮） */
const PAGE = 6

/** 主页模糊查词结果列表：单词 + 简介 + 已加入的词书。 */
export default function DictResultList({
  query,
  suggestions,
  presence,
  loading,
  error,
  onSelect,
  onRetry,
}: Props) {
  const [visible, setVisible] = useState(PAGE)
  const sentinelRef = useRef<HTMLDivElement>(null)

  // 换词就回到首屏条数（否则上一次展开的条数会带过来）
  useEffect(() => {
    setVisible(PAGE)
  }, [query])

  // 滚动到列表底部自动续 6 条：没有按钮，也不打断阅读。
  // rootMargin 提前 200px 触发，短列表在首屏内会连续补齐，直到 24 条全部出现。
  useEffect(() => {
    if (visible >= suggestions.length) return
    const node = sentinelRef.current
    if (!node) return
    if (typeof IntersectionObserver === 'undefined') {
      setVisible(suggestions.length)
      return
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return
        setVisible((current) => Math.min(current + PAGE, suggestions.length))
      },
      { rootMargin: '200px 0px' },
    )
    observer.observe(node)
    return () => observer.disconnect()
  }, [visible, suggestions.length])

  if (loading && suggestions.length === 0) {
    return (
      // 骨架与真实卡片同高（120px，实测值：标题行 + 两行摘要 min-h-[39px] + chips 行），结果到达时不塌陷/不弹跳
      <div className="space-y-2.5 animate-pulse" aria-hidden="true">
        {[0, 1, 2, 3].map((index) => (
          <div
            key={index}
            className="dict-result-skeleton px-4 py-3 space-y-2"
          >
            <div className="h-4 w-24 rounded bg-sand/60" />
            <div className="h-3 w-2/3 rounded bg-sand/40" />
            <div className="h-3 w-1/2 rounded bg-sand/40" />
            <div className="h-[22px] w-20 rounded-full bg-sand/30" />
          </div>
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <div className="dict-result-state min-h-[116px] px-6 py-6 text-center flex flex-col items-center justify-center">
        <p className="text-sm text-red-600">{error}</p>
        <button
          type="button"
          onClick={onRetry}
          className="mt-4 inline-flex items-center gap-1.5 min-h-11 px-5 rounded-full border border-charcoal/20 text-sm font-medium text-charcoal/75 hover:bg-white hover:border-charcoal/40 active:scale-95 transition-all"
        >
          <RefreshIcon className="w-3.5 h-3.5" />
          重试
        </button>
      </div>
    )
  }

  // useDictSuggest 在无建议时回退成「就查这个词」的一条：视觉上等同于空态
  const onlyEcho =
    suggestions.length === 1 &&
    !suggestions[0].summary &&
    suggestions[0].word.toLowerCase() === query.toLowerCase()

  if (suggestions.length === 0 || onlyEcho) {
    // 设计化空态：不写"没有查到"这类废话，直接给一个可执行入口
    return (
      <div className="dict-result-state dict-result-state--empty relative min-h-[116px] overflow-hidden px-6 py-7 text-center flex items-center justify-center">
        <span aria-hidden="true" className="pointer-events-none absolute -right-2 -top-4 font-serif text-7xl text-clay/15">
          Aa
        </span>
        <div className="relative">
          <button
            type="button"
            onClick={() => onSelect(query)}
            className="inline-flex items-center gap-2 min-h-11 px-5 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 active:scale-95 transition-all"
          >
            <SearchIcon className="w-4 h-4" />
            在词典中查看「{query}」
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-2.5">
      {suggestions.slice(0, visible).map((suggestion, index) => {
        const books = presence[suggestion.word.toLowerCase()] ?? []
        return (
          // 入场动画放外层：fill:both 会吃掉同元素的 hover transform。
          // 延迟封顶 6 格（330ms）：不封顶时「加载更多」的第 24 条要等 1.3s 才出来
          <div
            key={suggestion.word}
            className="animate-result-in"
            style={{ animationDelay: `${Math.min(index, 6) * 55}ms` }}
          >
            <button
              type="button"
              onClick={() => onSelect(suggestion.word)}
              aria-label={suggestion.word}
              className="dict-result-card group active:translate-y-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-clay/40"
            >
              <span className="min-w-0 flex-1">
                <span
                  style={{ viewTransitionName: transitionNameFor(suggestion.word) }}
                  className="vt-word block font-serif text-[17px] leading-snug text-charcoal truncate"
                >
                  {suggestion.word}
                </span>
                {/* line-clamp 自带 display:-webkit-box，不能再叠 block，否则截断失效、卡高不齐 */}
                {/* 摘要固定两行高（min-h-[39px]）：卡片等高、列表整齐，
                    骨架/空态才能用同一个数字占位，结果到达时不跳 */}
                {suggestion.summary && (
                  <span className="mt-1 block min-h-[39px] text-[13px] text-charcoal/55 leading-[1.5] line-clamp-2">
                    {suggestion.summary}
                  </span>
                )}
                {/* 常驻占位：词书归属异步返回时只填充内容，不改变卡片高度（避免 CLS） */}
                <span className="mt-1.5 flex flex-wrap gap-1.5 min-h-[22px]">
                  {books.map((book) => (
                    <span
                      key={book.book_id}
                      className="inline-block px-2.5 py-1 rounded-full bg-sage/50 text-charcoal/70 text-[11px] font-medium"
                    >
                      {book.book_name}
                    </span>
                  ))}
                </span>
              </span>
              <span className="shrink-0 mt-0.5 w-7 h-7 rounded-full bg-sand/40 flex items-center justify-center text-charcoal/30 group-hover:bg-charcoal group-hover:text-ivory transition-colors">
                <ArrowRightIcon className="w-3.5 h-3.5" />
              </span>
            </button>
          </div>
        )
      })}

      {/* 滚动哨兵：进入视口自动续 6 条；列表本身最多 24 条，不会无限长 */}
      {visible < suggestions.length && <div ref={sentinelRef} className="dict-result-sentinel" aria-hidden="true" />}
    </div>
  )
}
