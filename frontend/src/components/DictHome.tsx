import { useRef, useState } from 'react'
import { useDictSuggest } from '../hooks/useDictSuggest'
import { useWordPresence } from '../hooks/useWordPresence'
import { clearSearchHistory, loadSearchHistory, pushSearchHistory } from '../searchHistory'
import { transitionNameFor } from '../viewTransition'
import DictResultList from './DictResultList'
import { toast } from '../toast'
import { ArrowRightIcon, CloseIcon, SearchIcon } from './Icons'

interface Props {
  presenceVersion: number
  onOpenDetail: (word: string) => void
}

/** 查词分区：搜索、模糊结果、最近查询；不混入书架内容。 */
export default function DictHome({ presenceVersion, onOpenDetail }: Props) {
  const [input, setInput] = useState('')
  const searchRef = useRef<HTMLInputElement>(null)
  const [history, setHistory] = useState<string[]>(loadSearchHistory)
  const suggest = useDictSuggest(input)
  const presence = useWordPresence(suggest.suggestions.map((item) => item.word), presenceVersion)
  const historyPresence = useWordPresence(history, presenceVersion)
  const searching = input.trim().length >= 2

  function open(word: string) {
    setHistory(pushSearchHistory(word))
    onOpenDetail(word)
  }

  function submit(event: React.FormEvent) {
    event.preventDefault()
    if (input.trim().length < 2) return
    const first = suggest.suggestions[0]
    if (first) {
      open(first.word)
      return
    }
    suggest.refresh()
  }

  function clearHistory() {
    clearSearchHistory()
    setHistory([])
    toast('已清空最近查询')
  }

  return (
    <div className="animate-page-in">
      <header className="px-4 md:px-8">
        <div className="max-w-3xl mx-auto">
          {/* 入场动画放在外层：动画 fill 会覆盖同元素的 focus-within transform */}
          <div className="animate-fade-in-up">
            <div className="group relative overflow-hidden rounded-[2rem] border border-charcoal/5 bg-white/80 shadow-sm p-4 sm:p-5 transition-all duration-500 focus-within:-translate-y-0.5 focus-within:border-clay/30 focus-within:shadow-xl focus-within:shadow-charcoal/10">
              <form onSubmit={submit} className="relative w-full">
                {/* 整个胶囊可点：输入框自身 44 高，点图标/留白也聚焦 */}
                <div
                  onClick={(event) => {
                    if (!(event.target as HTMLElement).closest('button')) searchRef.current?.focus()
                  }}
                  className="flex items-center gap-2 bg-ivory rounded-full border border-charcoal/10 pl-5 pr-2 py-1 transition-colors focus-within:border-clay focus-within:ring-2 focus-within:ring-clay/25"
                >
                  <SearchIcon className="w-5 h-5 text-charcoal/35 shrink-0 transition-colors group-focus-within:text-clay" />
                  {/* 原生外观一律内联关掉（macOS / iOS 的 Chrome·Safari 会给 input 画一层内嵌边框，
                      CSS 类在个别版本上仍被 UA 样式压过；内联 style 一定生效） */}
                  <input
                    ref={searchRef}
                    value={input}
                    onChange={(event) => setInput(event.target.value)}
                    aria-label="搜索单词"
                    autoComplete="off"
                    autoCorrect="off"
                    autoCapitalize="off"
                    spellCheck={false}
                    style={{
                      WebkitAppearance: 'none',
                      appearance: 'none',
                      border: 0,
                      outline: 'none',
                      boxShadow: 'none',
                      background: 'transparent',
                    }}
                    className="flex-1 min-w-0 h-11 bg-transparent text-base text-charcoal focus:outline-none"
                  />
                  {input && (
                    <button
                      type="button"
                      onClick={() => setInput('')}
                      aria-label="清空"
                      className="shrink-0 min-h-11 min-w-11 -my-1 rounded-full flex items-center justify-center text-charcoal/40 hover:text-charcoal hover:bg-sand/60 active:bg-sand active:scale-95 transition-all"
                    >
                      <CloseIcon className="w-4 h-4" />
                    </button>
                  )}
                  <button
                    type="submit"
                    disabled={input.trim().length < 2}
                    className="shrink-0 min-h-11 px-6 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 hover:scale-[1.03] active:scale-95 transition-all disabled:opacity-40 disabled:hover:scale-100 disabled:cursor-not-allowed"
                  >
                    {suggest.loading ? '查询中…' : '查词'}
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      </header>

      {searching ? (
        <section className="max-w-3xl mx-auto px-4 md:px-8 mt-5">
          <DictResultList
            query={input.trim()}
            suggestions={suggest.suggestions}
            presence={presence}
            loading={suggest.loading}
            error={suggest.error}
            onSelect={open}
            onRetry={suggest.refresh}
          />
        </section>
      ) : history.length > 0 ? (
        <section className="max-w-3xl mx-auto px-4 md:px-8 mt-7">
          <div className="flex items-center gap-3 mb-4">
            <h2 className="font-serif text-xl text-charcoal">最近查询</h2>
            <span className="h-px flex-1 bg-charcoal/10" />
            <button
              type="button"
              onClick={clearHistory}
              className="min-h-11 min-w-11 px-3 rounded-full text-xs text-charcoal/50 hover:text-charcoal hover:bg-sand/40 active:bg-sand active:scale-95 transition-all"
            >
              清空
            </button>
          </div>
          <div className="space-y-3">
            {history.map((word, index) => (
              <div key={word} className="animate-fade-in-up" style={{ animationDelay: `${index * 45}ms` }}>
                <button
                  type="button"
                  onClick={() => open(word)}
                  aria-label={`查看 ${word} 的释义`}
                  className="group w-full min-h-14 text-left bg-white rounded-2xl border border-charcoal/5 shadow-sm px-4 py-3.5 flex items-center gap-3 hover:border-charcoal/15 hover:shadow-lg hover:-translate-y-0.5 active:translate-y-0 active:bg-sand/20 transition-all"
                >
                  <span className="w-7 text-[11px] tabular-nums text-charcoal/30">
                    {String(index + 1).padStart(2, '0')}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span
                      style={{ viewTransitionName: transitionNameFor(word) }}
                      className="vt-word block font-serif text-lg text-charcoal truncate transition-colors group-hover:text-clay"
                    >
                      {word}
                    </span>
                    {/* 常驻占位：归属异步返回时只填充内容，卡片高度不变（避免 CLS） */}
                    <span className="mt-1 flex flex-wrap gap-1 min-h-[22px]">
                      {(historyPresence[word.toLowerCase()] ?? []).map((book) => (
                        <span
                          key={book.book_id}
                          className="inline-block px-2 py-0.5 rounded-full bg-sage/50 text-charcoal/65 text-[11px] font-medium"
                        >
                          {book.book_name}
                        </span>
                      ))}
                    </span>
                  </span>
                  <span className="ml-auto shrink-0 w-8 h-8 rounded-full bg-sand/40 flex items-center justify-center text-charcoal/30 transition-all group-hover:bg-charcoal group-hover:text-ivory">
                    <ArrowRightIcon className="w-4 h-4" />
                  </span>
                </button>
              </div>
            ))}
          </div>
        </section>
      ) : (
        <section className="max-w-3xl mx-auto px-4 md:px-8 mt-8 animate-fade-in-up">
          <div className="relative overflow-hidden rounded-[2rem] border border-charcoal/5 bg-white/45 px-6 py-12 text-center">
            <div className="pointer-events-none absolute inset-0" aria-hidden="true">
              <span className="absolute left-[14%] top-7 font-serif text-3xl text-clay/25 -rotate-6">A</span>
              <span className="absolute right-[18%] top-10 font-serif text-2xl text-sage -rotate-12">b</span>
              <span className="absolute left-[24%] bottom-8 font-serif text-2xl text-charcoal/15 rotate-6">c</span>
              <span className="absolute right-[26%] bottom-6 font-serif text-4xl text-sand -rotate-3">d</span>
            </div>
            <div className="relative" aria-hidden="true">
              <span className="font-serif text-7xl text-clay/35">Aa</span>
              <span className="mx-auto mt-5 block h-px w-12 bg-clay/40" />
            </div>
          </div>
        </section>
      )}
    </div>
  )
}
