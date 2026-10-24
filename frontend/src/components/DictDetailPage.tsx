import { useEffect, useMemo, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { Definition, DictEntry, DictProviderId, Wordbook } from '../api'
import { getProviders } from '../dictCache'
import { useDictLookup } from '../hooks/useDictLookup'
import { useWordPresence } from '../hooks/useWordPresence'
import { lockScroll } from '../scrollLock'
import { transitionNameFor } from '../viewTransition'
import AmbientBackground from './AmbientBackground'
import DictBookStep from './DictBookStep'
import DictDefinitionStep from './DictDefinitionStep'
import { ArrowLeftIcon, GlobeIcon, PlusIcon, RefreshIcon, SpeakerIcon } from './Icons'
import { toast } from '../toast'

interface Props {
  word: string
  /** 全部词书：加入时先选书 */
  allBooks: Wordbook[]
  /** 保存后强制刷新存在性缓存 */
  presenceVersion: number
  onSave: (book: Wordbook, entry: DictEntry, definitions: Definition[]) => Promise<void>
  onCreateBook: () => void
  onClose: () => void
  /** 把「后退一步」注册给壳层：浏览器后退与 Esc 共用；返回 true = 已在步骤内消费 */
  onRegisterBack?: (handler: (() => boolean) | null) => void
}

type Step = 'detail' | 'books' | 'definitions'

interface DefinitionItem {
  index: number
  definition: Definition
}

interface DefinitionGroup {
  pos: string
  items: DefinitionItem[]
}

/** 每个词性默认展示的义项数（展开全部前） */
const PREVIEW_PER_GROUP = 3
/** 例句最多展示条数 */
const MAX_EXAMPLES = 3

function groupDefinitions(definitions: Definition[]): DefinitionGroup[] {
  const groups: DefinitionGroup[] = []
  definitions.forEach((definition, index) => {
    const pos = definition.pos.trim()
    const group = groups.find((item) => item.pos === pos)
    const item: DefinitionItem = { index, definition }
    if (group) {
      group.items.push(item)
    } else {
      groups.push({ pos, items: [item] })
    }
  })
  return groups
}

/** 词典详情页：按来源切换，不聚合各源释义；加入词书时先选书、再选义项。 */
export default function DictDetailPage({
  word,
  allBooks,
  presenceVersion,
  onSave,
  onCreateBook,
  onClose,
  onRegisterBack,
}: Props) {
  const [providers, setProviders] = useState<DictProviderId[]>([])
  const [picked, setPicked] = useState<DictProviderId | null>(null)
  const [step, setStep] = useState<Step>('detail')
  const [pickedBook, setPickedBook] = useState<Wordbook | null>(null)
  const [selected, setSelected] = useState<Set<number>>(new Set([0]))
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [showAll, setShowAll] = useState(false)
  const [speaking, setSpeaking] = useState(false)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const backButtonRef = useRef<HTMLButtonElement | null>(null)

  useEffect(() => {
    let alive = true
    getProviders()
      .then((list) => {
        if (alive) setProviders(list.filter((item) => item.enabled).map((item) => item.id))
      })
      .catch(() => {
        if (alive) setProviders([])
      })
    return () => {
      alive = false
    }
  }, [])

  // 详情是「打开即查」：防抖传 0，不让骨架白等 500ms
  const { entries, loading, error: lookupError, refresh } = useDictLookup(
    word,
    providers,
    providers.length > 0,
    undefined,
    0,
  )
  const presence = useWordPresence([word], presenceVersion)
  const books = presence[word.toLowerCase()] ?? []

  const entry = useMemo(() => {
    if (entries.length === 0) return null
    return entries.find((item) => item.provider === picked) ?? entries[0]
  }, [entries, picked])

  const groups = useMemo(() => groupDefinitions(entry?.definitions ?? []), [entry])

  /** 关闭/步骤回退：definitions → books → detail → 关闭；单本书时没有选书步骤 */
  function goBack() {
    if (step === 'definitions') {
      setStep(allBooks.length === 1 ? 'detail' : 'books')
      return
    }
    if (step === 'books') {
      setStep('detail')
      return
    }
    onClose()
  }

  const stepRef = useRef(step)
  stepRef.current = step
  const goBackRef = useRef(goBack)
  goBackRef.current = goBack

  // 把「后退一步」交给壳层（浏览器后退）；到详情首页返回 false 由壳层关闭
  useEffect(() => {
    if (!onRegisterBack) return
    onRegisterBack(() => {
      if (stepRef.current === 'detail') return false
      goBackRef.current()
      return true
    })
    return () => onRegisterBack(null)
  }, [onRegisterBack])

  // 对话框语义：进入时把焦点移进来、背景 inert，离开时还给触发元素
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null
    const root = document.getElementById('root')
    root?.setAttribute('inert', '')
    backButtonRef.current?.focus()
    return () => {
      root?.removeAttribute('inert')
      // 触发卡片在 .vt-home 里，关闭瞬间仍是 display:none；等一帧再还焦点（且只还一次）
      requestAnimationFrame(() => {
        if (previous && document.contains(previous)) {
          previous.focus({ preventScroll: true })
          return
        }
        // 触发卡片可能已随查询清空而卸载：退回到搜索框（可预期、可继续操作）
        document
          .querySelector<HTMLInputElement>('input[aria-label="搜索单词"]')
          ?.focus({ preventScroll: true })
      })
    }
  }, [])

  // Esc 与 Tab 焦点陷阱（只注册一次，通过 ref 读最新的 goBack）
  useEffect(() => {
    // 详情之上还压着弹窗（创建词书/确认框等）时，键盘事件归弹窗处理：
    // 否则一次 Esc 会把弹窗和详情一起关掉（Tab 也会被详情的陷阱抢回）。
    const hasOverlayDialog = () => {
      const dialogs = document.querySelectorAll<HTMLElement>('[role="dialog"][aria-modal="true"]')
      for (const dialog of dialogs) {
        if (dialog.classList.contains('vt-detail')) continue
        if (dialog.offsetParent !== null) return true
      }
      return false
    }
    const onKey = (event: KeyboardEvent) => {
      if (hasOverlayDialog()) return
      if (event.key === 'Escape') {
        event.stopPropagation()
        goBackRef.current()
        return
      }
      if (event.key !== 'Tab') return
      const root = scrollRef.current
      if (!root) return
      const nodes = Array.from(
        root.querySelectorAll<HTMLElement>(
          'button:not([disabled]), a[href], input:not([disabled]), [tabindex]:not([tabindex="-1"])',
        ),
      ).filter((el) => el.offsetParent !== null)
      if (nodes.length === 0) return
      const first = nodes[0]
      const last = nodes[nodes.length - 1]
      const current = document.activeElement
      if (event.shiftKey && (current === first || !root.contains(current))) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && (current === last || !root.contains(current))) {
        event.preventDefault()
        first.focus()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // 背景滚动锁：引用计数，叠开弹窗/多次快速开关都不会把 body 锁死
  useEffect(() => lockScroll(), [])

  useEffect(() => {
    setStep('detail')
    setPickedBook(null)
    setSelected(new Set([0]))
    setError('')
    setShowAll(false)
  }, [word])

  // 词条异步到达：已经站在释义步骤时，把刚拿到的义项全部勾上（默认全选）
  useEffect(() => {
    if (step !== 'definitions' || !entry) return
    setSelected(new Set(entry.definitions.map((_, index) => index)))
  }, [entry, step])

  useEffect(() => {
    // html 有 scroll-behavior: smooth，切步骤要即时回顶，不能从上一位置滑回来
    scrollRef.current?.scrollTo({ top: 0, behavior: 'instant' as ScrollBehavior })
  }, [step])

  function speak() {
    if (!('speechSynthesis' in window)) {
      toast('当前浏览器不支持朗读')
      return
    }
    window.speechSynthesis.cancel()
    const utterance = new SpeechSynthesisUtterance(entry?.word ?? word)
    utterance.lang = 'en-US'
    utterance.rate = 0.9
    utterance.onend = () => setSpeaking(false)
    utterance.onerror = () => {
      setSpeaking(false)
      toast('朗读失败，请稍后重试')
    }
    setSpeaking(true)
    toast(`正在朗读「${entry?.word ?? word}」`)
    window.speechSynthesis.speak(utterance)
  }

  /** 释义默认全选：进释义步骤时把当前词条的全部义项都勾上，用户只做「取消不要的」 */
  function allDefinitions(): Set<number> {
    return new Set((entry?.definitions ?? []).map((_, index) => index))
  }

  function chooseBook(book: Wordbook) {
    setPickedBook(book)
    setSelected(allDefinitions())
    setError('')
    setStep('definitions')
  }

  /** 加入词书入口：没有书先建；只有一本书直接进释义选择 */
  function startJoin() {
    if (allBooks.length === 0) {
      onCreateBook()
      return
    }
    if (allBooks.length === 1) {
      chooseBook(allBooks[0])
      return
    }
    setStep('books')
  }

  function toggleDefinition(index: number) {
    setSelected((current) => {
      const next = new Set(current)
      if (next.has(index)) next.delete(index)
      else next.add(index)
      return next
    })
  }

  async function save() {
    if (!pickedBook || !entry || selected.size === 0 || saving) return
    setSaving(true)
    setError('')
    try {
      const definitions = entry.definitions.filter((_, index) => selected.has(index))
      await onSave(pickedBook, entry, definitions)
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
    } finally {
      setSaving(false)
    }
  }

  const presentBookIds = new Set(books.map((book) => book.book_id))
  const allPresent = allBooks.length > 0 && allBooks.every((book) => presentBookIds.has(book.id))
  const title = step === 'detail' ? (entry?.word ?? word) : step === 'books' ? '选择单词书' : '选择释义'

  // 展开前按词性分组截断；编号按当前展示顺序连续（不沿用原始下标）
  const visibleGroups = useMemo(() => {
    const sliced = showAll
      ? groups
      : groups.map((group) => ({ ...group, items: group.items.slice(0, PREVIEW_PER_GROUP) }))
    let counter = 0
    return sliced.map((group) => ({
      ...group,
      items: group.items.map((item) => ({ ...item, number: (counter += 1) })),
    }))
  }, [groups, showAll])

  const shownCount = visibleGroups.reduce((count, group) => count + group.items.length, 0)
  const totalCount = entry?.definitions.length ?? 0

  return createPortal(
    <div
      ref={scrollRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="dict-detail-title"
      className="vt-detail fixed inset-0 z-50 bg-ivory overflow-y-auto animate-fade-in"
    >
      {/* 全屏详情是 portal 到 body 的独立层（z-50），页面壳层那层环境背景被它整片盖住：
          这里自带一层同一张底图（.ambient-stage 是 fixed，不随详情内滚动移动）。
          层级：底图 z-0 < 顶栏 z-10 / 内容(transform 自成层) < 底栏 z-20。 */}
      <AmbientBackground inline />
      <div className="sticky top-0 z-10 bg-ivory border-b border-charcoal/5 pt-[env(safe-area-inset-top)] animate-bar-down">
        <div className="max-w-3xl mx-auto px-4 md:px-8 h-14 flex items-center justify-between">
          <button
            ref={backButtonRef}
            type="button"
            onClick={goBack}
            aria-label="返回"
            className="w-11 h-11 -ml-2 rounded-full flex items-center justify-center text-charcoal/60 hover:text-charcoal hover:bg-sand/50 active:bg-sand active:scale-95 transition-all"
          >
            <ArrowLeftIcon className="w-4 h-4" />
          </button>
          <span
            id="dict-detail-title"
            className="min-w-0 truncate px-2 text-xs font-medium tracking-widest text-charcoal/55"
          >
            {title}
          </span>
          <span className="w-11" />
        </div>
      </div>

      {step === 'detail' && (
        <>
          <div className="max-w-3xl mx-auto px-4 md:px-8 pt-8 pb-32 animate-detail-in">
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <h2
                  // 只有「本次转场的词」才挂名字：无条件命名会让详情页自己多出一层快照，
                  // 与列表项名字冲突时浏览器会忽略两边，飞行直接消失。
                  style={{ viewTransitionName: transitionNameFor(word) }}
                  className="vt-word font-serif text-4xl text-charcoal break-words leading-tight animate-hero-in"
                >
                  {entry?.word ?? word}
                </h2>
                <div className="mt-2 flex items-center gap-1.5 flex-wrap">
                  {entry?.phonetic && (
                    <p className="text-base text-charcoal/60 break-words">{entry.phonetic}</p>
                  )}
                  <button
                    type="button"
                    onClick={speak}
                    aria-label="朗读"
                    aria-pressed={speaking}
                    className={`w-11 h-11 rounded-full flex items-center justify-center transition-all active:scale-95 ${
                      speaking
                        ? 'bg-charcoal text-ivory'
                        : 'text-charcoal/55 hover:text-charcoal hover:bg-sand/60 active:bg-sand'
                    }`}
                  >
                    <SpeakerIcon className="w-4 h-4" />
                  </button>
                </div>
                {books.length > 0 && (
                  <div className="mt-4 flex flex-wrap gap-1.5">
                    {books.map((book) => (
                      <span
                        key={book.book_id}
                        className="inline-block px-2.5 py-1 rounded-full bg-sage/50 text-charcoal/70 text-[11px] font-medium"
                      >
                        {book.book_name}
                      </span>
                    ))}
                  </div>
                )}
              </div>
              {entry?.source_url && (
                <a
                  href={entry.source_url}
                  target="_blank"
                  rel="noreferrer"
                  className="shrink-0 inline-flex items-center gap-1 min-h-11 px-2 text-[11px] text-charcoal/50 hover:text-clay active:text-clay active:scale-95 transition-all"
                >
                  <GlobeIcon className="w-3.5 h-3.5" />
                  来源
                </a>
              )}
            </div>

            {entries.length > 1 && (
              <div className="mt-6 flex flex-wrap gap-2">
                {entries.map((item, index) => {
                  const on = item.provider === entry?.provider
                  return (
                    <button
                      key={item.provider}
                      type="button"
                      onClick={() => {
                        setPicked(item.provider)
                        setShowAll(false)
                      }}
                      aria-pressed={on}
                      style={{ animationDelay: `${index * 55}ms` }}
                      className={`shrink-0 min-h-11 px-3.5 rounded-full border text-xs font-medium transition-all animate-result-in active:scale-95 ${
                        on
                          ? 'bg-charcoal text-ivory border-charcoal'
                          : 'bg-white text-charcoal/60 border-charcoal/10 hover:border-charcoal/30 active:bg-sand/40'
                      }`}
                    >
                      {item.label}
                    </button>
                  )
                })}
              </div>
            )}

            {loading ? (
              // 骨架按最终内容高度预留（释义 + 例句 + 来源 ≈ 380px），加载完成不塌陷
              <div className="mt-10 min-h-[380px] space-y-3 animate-pulse" aria-hidden="true">
                <div className="h-3 w-16 rounded bg-sand/60" />
                {[0, 1, 2, 3].map((index) => (
                  <div key={index} className="h-5 rounded bg-sand/50" />
                ))}
                <div className="pt-6 h-3 w-16 rounded bg-sand/60" />
                <div className="h-4 rounded bg-sand/40" />
                <div className="h-4 w-2/3 rounded bg-sand/40" />
              </div>
            ) : lookupError ? (
              <div className="mt-10 rounded-2xl border border-charcoal/10 bg-white/70 px-6 py-8 text-center">
                <p className="text-sm text-red-600">{lookupError}</p>
                <button
                  type="button"
                  onClick={refresh}
                  className="mt-4 inline-flex items-center gap-1.5 min-h-11 px-5 rounded-full border border-charcoal/20 text-sm font-medium text-charcoal/75 hover:bg-white hover:border-charcoal/40 active:scale-95 transition-all"
                >
                  <RefreshIcon className="w-3.5 h-3.5" />
                  重试
                </button>
              </div>
            ) : (
              entry && (
                <>
                  {entry.definitions.length > 0 && (
                    <section className="mt-8">
                      <h3 className="text-xs font-bold tracking-widest text-charcoal/55 uppercase">
                        释义
                      </h3>
                      <div className="mt-3 space-y-4">
                        {visibleGroups.map((group) => (
                          <div key={group.pos || 'none'}>
                            {group.pos && (
                              <span className="inline-block mb-1.5 px-2 py-0.5 rounded-md bg-sand/60 text-charcoal/65 text-[11px] font-medium">
                                {group.pos}
                              </span>
                            )}
                            <ol className="space-y-1.5">
                              {group.items.map(({ index, definition, number }) => (
                                <li
                                  key={index}
                                  style={{ animationDelay: `${Math.min(number - 1, 8) * 35}ms` }}
                                  className="flex items-start gap-2.5 text-[15px] text-charcoal/85 leading-relaxed break-words animate-result-in"
                                >
                                  <span className="shrink-0 w-5 text-right text-xs text-charcoal/35 tabular-nums">
                                    {number}.
                                  </span>
                                  <span className="min-w-0">{definition.meaning}</span>
                                </li>
                              ))}
                            </ol>
                          </div>
                        ))}
                      </div>
                      {totalCount > shownCount && (
                        <button
                          type="button"
                          onClick={() => {
                            setShowAll(true)
                            toast(`已展开全部 ${totalCount} 条释义`)
                          }}
                          className="mt-4 min-h-11 px-4 rounded-full border border-charcoal/15 text-xs font-medium text-charcoal/65 hover:border-charcoal/30 hover:text-charcoal active:bg-sand/40 active:scale-95 transition-all"
                        >
                          展开全部 {totalCount} 条
                        </button>
                      )}
                    </section>
                  )}

                  {entry.examples.length > 0 && (
                    <section className="mt-10">
                      <h3 className="text-xs font-bold tracking-widest text-charcoal/55 uppercase">
                        例句
                      </h3>
                      <div className="mt-3 space-y-4">
                        {entry.examples.slice(0, MAX_EXAMPLES).map((example, index) => (
                          <div
                            key={index}
                            style={{ animationDelay: `${index * 70}ms` }}
                            className="border-l-2 border-clay/40 pl-4 text-[15px] text-charcoal/75 leading-relaxed break-words animate-result-in"
                          >
                            {example.text}
                            {example.translation && (
                              <span className="block mt-1 text-sm text-charcoal/55">
                                {example.translation}
                              </span>
                            )}
                          </div>
                        ))}
                      </div>
                    </section>
                  )}
                </>
              )
            )}
          </div>

          <div className="fixed bottom-0 inset-x-0 z-20 border-t border-charcoal/10 bg-ivory">
            <div className="max-w-3xl mx-auto px-4 md:px-8 py-3 pb-[calc(0.75rem+env(safe-area-inset-bottom))]">
              <button
                type="button"
                onClick={startJoin}
                disabled={loading || !entry || allPresent}
                className="w-full inline-flex items-center justify-center gap-2 min-h-12 px-6 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 active:scale-[0.99] transition-all disabled:bg-charcoal/15 disabled:text-charcoal/55 disabled:hover:bg-charcoal/15"
              >
                <PlusIcon className="w-4 h-4" />
                {loading ? '加载中…' : allBooks.length === 0 ? '创建单词书' : allPresent ? '已收录' : '加入词书'}
              </button>
            </div>
          </div>
        </>
      )}

      {step === 'books' && (
        <DictBookStep
          books={allBooks}
          presentBookIds={presentBookIds}
          onChoose={chooseBook}
          onCreate={onCreateBook}
        />
      )}

      {step === 'definitions' && entry && pickedBook && (
        <DictDefinitionStep
          entry={entry}
          book={pickedBook}
          selected={selected}
          saving={saving}
          error={error}
          onToggle={toggleDefinition}
          onSave={save}
        />
      )}
    </div>,
    document.body,
  )
}
