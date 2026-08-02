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
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [error, setError] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Word | null>(null)

  // 分页缓存：key = `${bookId}:${page}:${size}`，避免重复请求；预取相邻页
  const cache = useRef<Map<string, Page<Word>>>(new Map())

  // 书信息只加载一次（不随翻页重复请求）
  useEffect(() => {
    let alive = true
    wordbooks
      .list()
      .then((bs) => {
        if (alive) setBook(bs.find((b) => b.id === bookId) ?? null)
      })
      .catch((e) => setError(e instanceof Error ? e.message : '加载失败'))
    return () => {
      alive = false
    }
  }, [bookId])

  const loadPage = useCallback(
    async (p: number, size: number, opts?: { prefetch?: boolean }) => {
      const key = `${bookId}:${p}:${size}`
      const cached = cache.current.get(key)
      if (cached) {
        setData(cached)
        return
      }
      try {
        setError('')
        const paged = await words.list(bookId, p, size)
        cache.current.set(key, paged)
        setData(paged)
        if (opts?.prefetch !== false) {
          if (p > 1 && !cache.current.has(`${bookId}:${p - 1}:${size}`)) {
            words
              .list(bookId, p - 1, size)
              .then((r) => cache.current.set(`${bookId}:${p - 1}:${size}`, r))
              .catch(() => {})
          }
          if (p < paged.total_pages && !cache.current.has(`${bookId}:${p + 1}:${size}`)) {
            words
              .list(bookId, p + 1, size)
              .then((r) => cache.current.set(`${bookId}:${p + 1}:${size}`, r))
              .catch(() => {})
          }
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : '加载失败')
      }
    },
    [bookId],
  )

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

  /** 增删改后：清缓存重载当前页 + 刷新书信息 */
  async function onMutated() {
    cache.current.clear()
    await loadPage(page, pageSize, { prefetch: true })
    try {
      const bs = await wordbooks.list()
      setBook(bs.find((b) => b.id === bookId) ?? null)
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
          <div className="bg-white/70 backdrop-blur-md border border-white/40 shadow-sm rounded-full px-6 py-3 flex items-center gap-3">
            <button
              onClick={onBack}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium text-charcoal/70 hover:bg-sand/40 hover:text-charcoal transition-colors"
            >
              <ArrowLeftIcon className="w-4 h-4" />
              返回
            </button>
            <div className="flex items-center gap-2 min-w-0">
              <div className="w-7 h-7 rounded-full bg-sage flex items-center justify-center text-white shrink-0">
                <BookIcon className="w-3.5 h-3.5" />
              </div>
              <h1 className="font-serif text-base text-charcoal truncate">{book ? book.name : '…'}</h1>
              {data && (
                <span className="shrink-0 text-xs text-charcoal/40 tabular-nums">{data.total} 词</span>
              )}
            </div>
            <div className="ml-auto flex items-center gap-2.5">
              <div className="bg-sand/30 rounded-full p-1 flex" role="tablist" aria-label="视图模式">
                {(['paper', 'list'] as const).map((m) => (
                  <button
                    key={m}
                    role="tab"
                    aria-selected={mode === m}
                    onClick={() => setMode(m)}
                    className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 ${
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
              {/* 阅读设置 */}
              <div className="relative">
                <button
                  onClick={() => setSettingsOpen((o) => !o)}
                  className={`w-8 h-8 rounded-full flex items-center justify-center transition-all ${
                    settingsOpen
                      ? 'bg-charcoal text-ivory'
                      : 'bg-sand/40 text-charcoal/70 hover:bg-sand/70'
                  }`}
                  aria-label="阅读设置"
                  aria-expanded={settingsOpen}
                >
                  <SettingsIcon className="w-4 h-4" />
                </button>
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
              <button
                onClick={handleOpenCreate}
                className="inline-flex items-center gap-1.5 bg-charcoal text-ivory px-4 py-2 rounded-full text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10"
              >
                <PlusIcon className="w-4 h-4" />
                添加单词
              </button>
            </div>
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 md:px-8 pt-32 pb-16">
        {error && <p className="text-red-600 text-center mb-6">{error}</p>}

        {mode === 'paper' ? (
          <PaperBookView
            data={data}
            page={page}
            onPrev={goPrev}
            onNext={goNext}
            onAddFirst={handleOpenCreate}
            fontScale={fontScale}
          />
        ) : (
          <WordTable
            data={data}
            page={page}
            onPrev={goPrev}
            onNext={goNext}
            onEdit={handleOpenEdit}
            onDelete={handleDelete}
          />
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
