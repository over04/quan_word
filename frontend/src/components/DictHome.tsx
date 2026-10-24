import { useRef, useState } from 'react'
import { useDictSuggest } from '../hooks/useDictSuggest'
import { useWordPresence } from '../hooks/useWordPresence'
import { clearSearchHistory, loadSearchHistory, pushSearchHistory } from '../searchHistory'
import { transitionNameFor } from '../viewTransition'
import DictResultList from './DictResultList'
import LexiconMark from './LexiconMark'
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
    <div className={`lex-home ${searching ? 'is-searching' : ''} animate-page-in`}>
      <header className="lex-hero px-4">
        <span className="lex-kicker">LEXICON</span>
        <h1 className="sr-only">查词</h1>
        <div className="lex-mark-wrap">
          <LexiconMark className="lex-mark" />
        </div>

        <div className="lex-search-wrap animate-fade-in-up">
          <div className="lex-search">
            <form onSubmit={submit} className="w-full">
              {/* 整个胶囊可点：输入框自身 44 高，点图标/留白也聚焦 */}
              <div
                className="lex-search-row"
                onClick={(event) => {
                  if (!(event.target as HTMLElement).closest('button')) searchRef.current?.focus()
                }}
              >
                <span className="lex-search-seal" aria-hidden="true">
                  <SearchIcon className="w-5 h-5" />
                </span>
                {/* 原生外观一律内联关掉（macOS / iOS 的 Chrome·Safari 会给 input 画一层内嵌边框） */}
                <input
                  ref={searchRef}
                  value={input}
                  onChange={(event) => setInput(event.target.value)}
                  aria-label="搜索单词"
                  placeholder="输入单词"
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
                  className="lex-input"
                />
                {input && (
                  <button type="button" onClick={() => setInput('')} aria-label="清空" className="lex-clear">
                    <CloseIcon className="w-4 h-4" />
                  </button>
                )}
                <button type="submit" disabled={input.trim().length < 2} className="lex-submit">
                  <span>{suggest.loading ? '查询中…' : '查词'}</span>
                  <ArrowRightIcon className="w-4 h-4" />
                </button>
              </div>
            </form>
          </div>
        </div>

        {!searching && history.length > 0 && (
          <div className="lex-recent-chips">
            <span className="lex-recent-label">最近</span>
            {history.map((word) => (
              <button
                key={word}
                type="button"
                onClick={() => open(word)}
                aria-label={`查看 ${word} 的释义`}
                className="lex-chip"
              >
                <span style={{ viewTransitionName: transitionNameFor(word) }} className="vt-word">
                  {word}
                </span>
              </button>
            ))}
            <button type="button" onClick={clearHistory} className="lex-chip-clear">
              清空
            </button>
          </div>
        )}
      </header>

      {searching && (
        <section className="lex-results px-4 mt-8">
          <div className="lex-section-head">
            <div className="lex-section-copy">
              <span className="lex-section-kicker">RESULTS</span>
              <h2 className="lex-section-title">{input.trim()}</h2>
            </div>
            <span className="lex-section-rule" aria-hidden="true" />
            <span className="lex-section-meta">
              {suggest.loading ? '检索中' : `${suggest.suggestions.length} 条`}
            </span>
          </div>
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
      )}
    </div>
  )
}
