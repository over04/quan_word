import { useCallback, useEffect, useRef, useState } from 'react'
import { wordbooks, words, type Page, type Word, type Wordbook } from '../api'
import PaperBookView from '../components/PaperBookView'
import WordTable from '../components/WordTable'
import WordFormModal from '../components/WordFormModal'
import SettingsPanel from '../components/SettingsPanel'
import {
  ArrowLeftIcon,
  BookIcon,
  ListIcon,
  PaperIcon,
  PlusIcon,
  SettingsIcon,
} from '../components/Icons'

interface Props {
  bookId: number
  onBack: () => void
}

function loadPageSize(): number {
  const v = Number(localStorage.getItem('qw_page_size'))
  return Number.isFinite(v) && v >= 10 && v <= 200 ? v : 30
}

function loadFontScale(): number {
  const v = Number(localStorage.getItem('qw_font_scale'))
  return v >= 12 && v <= 28 ? v : 20
}

export default function WordbookDetail({ bookId, onBack }: Props) {
  const [book, setBook] = useState<Wordbook | null>(null)
  const [data, setData] = useState<Page<Word> | null>(null)
  const [mode, setMode] = useState<'paper' | 'list'>('paper')
  const [page, setPage] = useState(1)
  const [pageSize, setPageSize] = useState<number>(loadPageSize)
  const [fontScale, setFontScale] = useState<number>(loadFontScale)
  // 一键模糊：整页单词（含音标）/ 释义（导航栏按钮切换）
  const [coverWord, setCoverWord] = useState(false)
  const [coverDef, setCoverDef] = useState(false)
  // 打乱：seed 非空 = 随机顺序浏览（确定性，翻页稳定）
  const [shuffleSeed, setShuffleSeed] = useState<string | null>(null)
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [error, setError] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Word | null>(null)
  // 列表模式刷新信号：增删改后递增，WordTable 重新查询
  const [listRefresh, setListRefresh] = useState(0)

  // 分页缓存：key = `${bookId}:${page}:${size}`，避免重复请求；预取相邻页
  const cache = useRef<Map<string, Page<Word>>>(new Map())
  // 相邻页数据（预取结果），供纸质书滑动翻页时直接可见
  const [neighbor, setNeighbor] = useState<{ prev: Page<Word> | null; next: Page<Word> | null }>({
    prev: null,
    next: null,
  })

  // 书信息只加载一次（不随翻页重复请求）；单书接口，不再拉全量列表
  useEffect(() => {
    let alive = true
    wordbooks
      .get(bookId)
      .then((b) => {
        if (alive) setBook(b)
      })
      .catch((e) => setError(e instanceof Error ? e.message : '加载失败'))
    return () => {
      alive = false
    }
  }, [bookId])

  const loadPage = useCallback(
    async (p: number, size: number, opts?: { prefetch?: boolean; seed?: string | null }) => {
      // 排序参数影响内容，缓存 key 必须包含 seed
      const seed = opts?.seed !== undefined ? opts.seed : shuffleSeed
      const key = `${bookId}:${p}:${size}:${seed ?? ''}`
      const cached = cache.current.get(key)
      if (cached) {
        setData(cached)
        if (opts?.prefetch !== false) prefetchNeighbors(p, size, cached.total_pages, seed)
        syncNeighbor(p, size, seed)
        return
      }
      try {
        setError('')
        const paged = await words.list(bookId, p, size, seed ? { order: 'random', seed } : undefined)
        cache.current.set(key, paged)
        setData(paged)
        if (opts?.prefetch !== false) prefetchNeighbors(p, size, paged.total_pages, seed)
        syncNeighbor(p, size, seed)
      } catch (e) {
        setError(e instanceof Error ? e.message : '加载失败')
      }
    },
    [bookId, prefetchNeighbors, shuffleSeed],
  )

  /** 预取相邻页（缓存命中与网络加载两条路径都执行，保证滑动翻页时相邻页可见） */
  function prefetchNeighbors(p: number, size: number, totalPages: number, seed: string | null) {
    const keyOf = (pp: number) => `${bookId}:${pp}:${size}:${seed ?? ''}`
    if (p > 1 && !cache.current.has(keyOf(p - 1))) {
      const pk = keyOf(p - 1)
      words
        .list(bookId, p - 1, size, seed ? { order: 'random', seed } : undefined)
        .then((r) => {
          cache.current.set(pk, r)
          setNeighbor((n) => ({ ...n, prev: r }))
        })
        .catch(() => {})
    }
    if (p < totalPages && !cache.current.has(keyOf(p + 1))) {
      const nk = keyOf(p + 1)
      words
        .list(bookId, p + 1, size, seed ? { order: 'random', seed } : undefined)
        .then((r) => {
          cache.current.set(nk, r)
          setNeighbor((n) => ({ ...n, next: r }))
        })
        .catch(() => {})
    }
  }

  /** 从缓存同步相邻页数据（翻页后相邻页已预取或已缓存） */
  function syncNeighbor(p: number, size: number, seed: string | null) {
    const keyOf = (pp: number) => `${bookId}:${pp}:${size}:${seed ?? ''}`
    setNeighbor({
      prev: cache.current.get(keyOf(p - 1)) ?? null,
      next: cache.current.get(keyOf(p + 1)) ?? null,
    })
  }

  /** 打乱 / 恢复顺序：换 seed 并清缓存重载第 1 页 */
  function toggleShuffle() {
    const seed = shuffleSeed ? null : String(Date.now())
    setShuffleSeed(seed)
    cache.current.clear()
    setPage(1)
    loadPage(1, pageSize, { prefetch: true, seed })
  }

  // 首次加载第 1 页（含预取）
  useEffect(() => {
    loadPage(1, pageSize, { prefetch: true })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  function changePageSize(n: number) {
    localStorage.setItem('qw_page_size', String(n))
    cache.current.clear()
    setPageSize(n)
    setPage(1)
    loadPage(1, n, { prefetch: true })
  }

  function changeFontScale(s: number) {
    localStorage.setItem('qw_font_scale', String(s))
    setFontScale(s)
  }

  function goPrev() {
    if (page <= 1) return
    const np = page - 1
    setPage(np)
    loadPage(np, pageSize)
  }

  function goNext() {
    if (!data || page >= data.total_pages) return
    const np = page + 1
    setPage(np)
    loadPage(np, pageSize)
  }

  /** 增删改后：清缓存重载当前页 + 刷新书信息 + 触发列表模式重新查询 */
  async function onMutated() {
    cache.current.clear()
    await loadPage(page, pageSize, { prefetch: true })
    setListRefresh((k) => k + 1)
    try {
      const b = await wordbooks.get(bookId)
      setBook(b)
    } catch {
      // 书信息刷新失败不影响单词列表
    }
  }

  function handleOpenCreate() {
    setEditing(null)
    setFormOpen(true)
  }

  function handleOpenEdit(w: Word) {
    setEditing(w)
    setFormOpen(true)
  }

  async function handleDelete(w: Word) {
    if (!window.confirm(`确定删除「${w.spelling}」吗？`)) return
    try {
      await words.remove(w.id)
      await onMutated()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    }
  }

  return (
    <div className="min-h-screen">
      {/* 悬浮胶囊导航 */}
      <nav className="fixed w-full top-0 z-40 py-4 px-4 md:px-8">
        <div className="max-w-7xl mx-auto">
          {/* 外层 relative：设置浮层锚点（在滚动容器外，避免被 overflow-x-auto 裁剪） */}
          <div className="relative">
          <div className="bg-white/70 backdrop-blur-md border border-white/40 shadow-sm rounded-full px-6 py-3 flex items-center gap-3 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <button
              onClick={onBack}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium text-charcoal/70 hover:bg-sand/40 hover:text-charcoal transition-colors whitespace-nowrap shrink-0"
            >
              <ArrowLeftIcon className="w-4 h-4" />
              返回
            </button>
            <div className="w-7 h-7 rounded-full bg-sage flex items-center justify-center text-white shrink-0">
              <BookIcon className="w-3.5 h-3.5" />
            </div>
            <div className="flex items-center min-w-0 flex-1">
              <h1 className="font-serif text-base text-charcoal truncate">{book ? book.name : '…'}</h1>
            </div>
            {data && (
              <span className="shrink-0 text-xs text-charcoal/40 tabular-nums whitespace-nowrap">{data.total} 词</span>
            )}
            <div className="ml-auto flex items-center gap-2.5 shrink-0">
              <div className="bg-sand/30 rounded-full p-1 flex shrink-0" role="tablist" aria-label="视图模式">
                {(['paper', 'list'] as const).map((m) => (
                  <button
                    key={m}
                    role="tab"
                    aria-selected={mode === m}
                    onClick={() => setMode(m)}
                    className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap ${
                      mode === m
                        ? 'bg-charcoal text-ivory shadow-md'
                        : 'text-charcoal/70 hover:text-charcoal'
                    }`}
                  >
                    {m === 'paper' ? <PaperIcon className="w-4 h-4" /> : <ListIcon className="w-4 h-4" />}
                    {m === 'paper' ? '纸质书' : '列表'}
                  </button>
                ))}
              </div>
              {/* 显示设置（仅纸质书模式，位置在模式切换旁） */}
              {mode === 'paper' && (
                <button
                  onClick={() => setSettingsOpen((o) => !o)}
                  className={`w-8 h-8 rounded-full flex items-center justify-center transition-all shrink-0 ${
                    settingsOpen
                      ? 'bg-charcoal text-ivory'
                      : 'bg-sand/40 text-charcoal/70 hover:bg-sand/70'
                  }`}
                  aria-label="显示设置"
                  aria-expanded={settingsOpen}
                >
                  <SettingsIcon className="w-4 h-4" />
                </button>
              )}
              {/* 一键模糊（纸质书模式）：单词含音标 / 释义 */}
              {mode === 'paper' && (
                <>
                <div className="bg-sand/30 rounded-full p-1 flex shrink-0" role="group" aria-label="一键模糊">
                  <button
                    aria-pressed={coverWord}
                    onClick={() => setCoverWord((v) => !v)}
                    className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap ${
                      coverWord ? 'bg-charcoal text-ivory shadow-md' : 'text-charcoal/70 hover:text-charcoal'
                    }`}
                  >
                    单词
                  </button>
                  <button
                    aria-pressed={coverDef}
                    onClick={() => setCoverDef((v) => !v)}
                    className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap ${
                      coverDef ? 'bg-charcoal text-ivory shadow-md' : 'text-charcoal/70 hover:text-charcoal'
                    }`}
                  >
                    释义
                  </button>
                </div>
                {/* 打乱：随机顺序浏览（确定性 seed，翻页稳定） */}
                <button
                  aria-pressed={shuffleSeed !== null}
                  onClick={toggleShuffle}
                  className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap shrink-0 ${
                    shuffleSeed !== null
                      ? 'bg-charcoal text-ivory shadow-md'
                      : 'text-charcoal/70 hover:text-charcoal'
                  }`}
                >
                  打乱
                </button>
                </>
              )}
              <button
                onClick={handleOpenCreate}
                className="inline-flex items-center gap-1.5 bg-charcoal text-ivory px-4 py-2 rounded-full text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 whitespace-nowrap shrink-0"
              >
                <PlusIcon className="w-4 h-4" />
                添加单词
              </button>
            </div>
          </div>
          {/* 设置面板：渲染在滚动容器外，右上角对齐胶囊 */}
          {settingsOpen && (
            <SettingsPanel
              pageSize={pageSize}
              fontScale={fontScale}
              onChangePageSize={changePageSize}
              onChangeFontScale={changeFontScale}
              onClose={() => setSettingsOpen(false)}
            />
          )}
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 md:px-8 pt-32 pb-16">
        {error && <p className="text-red-600 text-center mb-6">{error}</p>}

        {mode === 'paper' ? (
          <div key="paper" className="animate-fade-in-up">
            <PaperBookView
              data={data}
              page={page}
              onPrev={goPrev}
              onNext={goNext}
              onAddFirst={handleOpenCreate}
              fontScale={fontScale}
              coverWord={coverWord}
              coverDef={coverDef}
              prevPage={neighbor.prev}
              nextPage={neighbor.next}
            />
          </div>
        ) : (
          <div key="list" className="animate-fade-in-up">
            <WordTable
              bookId={bookId}
              refreshKey={listRefresh}
              onEdit={handleOpenEdit}
              onDelete={handleDelete}
            />
          </div>
        )}
      </main>

      {formOpen && (
        <WordFormModal
          bookId={bookId}
          initial={editing}
          onClose={() => setFormOpen(false)}
          onSaved={async () => {
            setFormOpen(false)
            await onMutated()
          }}
        />
      )}
    </div>
  )
}
