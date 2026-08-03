import { useCallback, useEffect, useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import {
  tags as tagApi,
  wordbooks,
  words,
  type Page,
  type Tag,
  type Word,
  type Wordbook,
} from '../api'
import PaperBookView from '../components/PaperBookView'
import TagManageModal from '../components/TagManageModal'
import ConfirmModal from '../components/ConfirmModal'
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
  TagIcon,
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

/** 遮挡状态：基线（整书全隐藏）+ 手动例外（点过的词，绝对隐藏状态）。按书持久化 */
interface CoverState {
  hideAllWord: boolean
  hideAllDef: boolean
  wordDiff: Record<number, boolean>
  defDiff: Record<number, boolean>
}

const DEFAULT_COVER: CoverState = { hideAllWord: false, hideAllDef: false, wordDiff: {}, defDiff: {} }

function loadCover(bookId: number): CoverState {
  try {
    const raw = localStorage.getItem(`qw_cover_${bookId}`)
    if (!raw) return DEFAULT_COVER
    const p = JSON.parse(raw)
    if (typeof p !== 'object' || p === null) return DEFAULT_COVER
    return {
      hideAllWord: p.hideAllWord === true,
      hideAllDef: p.hideAllDef === true,
      wordDiff: p.wordDiff && typeof p.wordDiff === 'object' ? p.wordDiff : {},
      defDiff: p.defDiff && typeof p.defDiff === 'object' ? p.defDiff : {},
    }
  } catch {
    return DEFAULT_COVER
  }
}

/** 标签筛选（按书持久化）：非法值过滤，损坏回退空 */
function loadFilterTags(bookId: number): number[] {
  try {
    const raw = localStorage.getItem(`qw_filter_tags_${bookId}`)
    if (!raw) return []
    const p = JSON.parse(raw)
    if (!Array.isArray(p)) return []
    return p.filter((x) => Number.isFinite(x)).map(Number)
  } catch {
    return []
  }
}

/** 打乱 seed（按书持久化）：刷新后恢复同一随机顺序 */
function loadShuffleSeed(bookId: number): string | null {
  try {
    const v = localStorage.getItem(`qw_shuffle_seed_${bookId}`)
    return v ? v : null
  } catch {
    return null
  }
}

/** 页码（按书持久化）：刷新后恢复上次浏览页；非法值回退第 1 页 */
function loadPageNo(bookId: number): number {
  const v = Number(localStorage.getItem(`qw_page_${bookId}`))
  return Number.isFinite(v) && v >= 1 ? Math.floor(v) : 1
}

export default function WordbookDetail({ bookId, onBack }: Props) {
  const [book, setBook] = useState<Wordbook | null>(null)
  const [data, setData] = useState<Page<Word> | null>(null)
  const [mode, setMode] = useState<'paper' | 'list'>('paper')
  const [page, setPage] = useState<number>(() => loadPageNo(bookId))
  useEffect(() => {
    localStorage.setItem(`qw_page_${bookId}`, String(page))
  }, [page, bookId])
  const [pageSize, setPageSize] = useState<number>(loadPageSize)
  const [fontScale, setFontScale] = useState<number>(loadFontScale)
  // 遮挡：基线（整书全隐藏）+ 手动例外（点过的词）；按书持久化到 localStorage
  const [cover, setCover] = useState<CoverState>(() => loadCover(bookId))
  // 任何变更即时写回；bookId 在组件生命周期内不变（每次进入书重新挂载）
  useEffect(() => {
    localStorage.setItem(`qw_cover_${bookId}`, JSON.stringify(cover))
  }, [cover, bookId])
  // 打乱：seed 非空 = 随机顺序浏览（确定性，翻页稳定）；按书持久化，刷新后恢复原随机顺序
  const [shuffleSeed, setShuffleSeed] = useState<string | null>(() => loadShuffleSeed(bookId))
  useEffect(() => {
    if (shuffleSeed) localStorage.setItem(`qw_shuffle_seed_${bookId}`, shuffleSeed)
    else localStorage.removeItem(`qw_shuffle_seed_${bookId}`)
  }, [shuffleSeed, bookId])
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [error, setError] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Word | null>(null)
  // 待确认删除的词（ConfirmModal 替换原生 confirm）
  const [pendingDelete, setPendingDelete] = useState<Word | null>(null)
  // 标签：该书全部标签 + 筛选（多选交集，两模式共享）+ 管理弹窗；筛选按书持久化，刷新后恢复
  const [tags, setTags] = useState<Tag[]>([])
  const [filterTagIds, setFilterTagIds] = useState<number[]>(() => loadFilterTags(bookId))
  useEffect(() => {
    localStorage.setItem(`qw_filter_tags_${bookId}`, JSON.stringify(filterTagIds))
  }, [filterTagIds, bookId])
  const [tagFilterOpen, setTagFilterOpen] = useState(false)
  const [manageTagsOpen, setManageTagsOpen] = useState(false)
  // 列表模式刷新信号：增删改后递增，WordTable 重新查询
  const [listRefresh, setListRefresh] = useState(0)

  // 分页缓存：key = `${bookId}:${page}:${size}`，避免重复请求；预取相邻页
  const cache = useRef<Map<string, Page<Word>>>(new Map())
  // 请求序号：翻页/筛选/打乱快速连续操作时丢弃过期响应（最后一次调用生效）
  const seqRef = useRef(0)
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

  // 进入书：滚动复位到顶部（body 滚动位置会从主页继承，否则从书页中间开始看）
  useEffect(() => {
    window.scrollTo(0, 0)
  }, [])

  // 标签列表只加载一次（筛选与表单共用）
  useEffect(() => {
    let alive = true
    tagApi
      .list(bookId)
      .then((t) => {
        if (alive) setTags(t)
      })
      .catch((e) => setError(e instanceof Error ? e.message : '加载失败'))
    return () => {
      alive = false
    }
  }, [bookId])

  /** 取一页数据（缓存命中同步返回；请求带序号，过期响应丢弃，不写缓存不报错） */
  async function fetchPage(
    p: number,
    size: number,
    seed: string | null,
    tagIds: number[],
  ): Promise<Page<Word> | null> {
    const key = `${bookId}:${p}:${size}:${seed ?? ''}:${tagIds.join(',')}`
    const seq = ++seqRef.current
    const cached = cache.current.get(key)
    if (cached) return cached
    const tagParam = tagIds.length > 0 ? tagIds.join(',') : undefined
    try {
      const paged = await words.list(bookId, p, size, {
        ...(seed ? { order: 'random', seed } : {}),
        ...(tagParam ? { tag: tagParam } : {}),
      })
      if (seq !== seqRef.current) return null // 过期响应：丢弃
      cache.current.set(key, paged)
      return paged
    } catch (e) {
      if (seq !== seqRef.current) return null
      setError(e instanceof Error ? e.message : '加载失败')
      return null
    }
  }

  const loadPage = useCallback(
    async (p: number, size: number, opts?: { prefetch?: boolean; seed?: string | null; tagIds?: number[] }) => {
      const seed = opts?.seed !== undefined ? opts.seed : shuffleSeed
      const tagIds = opts?.tagIds ?? filterTagIds
      // 空页回退：持久化页码越界（词被删页数减少）或删除后当前页删空 → 逐页回退到有数据的页
      let target = p
      let paged = await fetchPage(target, size, seed, tagIds)
      while (paged && paged.items.length === 0 && target > 1) {
        target -= 1
        setPage(target)
        paged = await fetchPage(target, size, seed, tagIds)
      }
      if (!paged) return // 过期或失败：不更新 UI
      setError('')
      setData(paged)
      if (opts?.prefetch !== false) prefetchNeighbors(target, size, paged.total_pages, seed, tagIds)
      syncNeighbor(target, size, seed, tagIds)
    },
    [bookId, prefetchNeighbors, shuffleSeed, filterTagIds],
  )

  /** 预取相邻页（缓存命中与网络加载两条路径都执行，保证滑动翻页时相邻页可见） */
  function prefetchNeighbors(
    p: number,
    size: number,
    totalPages: number,
    seed: string | null,
    tagIds: number[],
  ) {
    const tagParam = tagIds.length > 0 ? tagIds.join(',') : undefined
    const keyOf = (pp: number) => `${bookId}:${pp}:${size}:${seed ?? ''}:${tagIds.join(',')}`
    if (p > 1 && !cache.current.has(keyOf(p - 1))) {
      const pk = keyOf(p - 1)
      words
        .list(bookId, p - 1, size, {
          ...(seed ? { order: 'random', seed } : {}),
          ...(tagParam ? { tag: tagParam } : {}),
        })
        .then((r) => {
          cache.current.set(pk, r)
          setNeighbor((n) => ({ ...n, prev: r }))
        })
        .catch(() => {})
    }
    if (p < totalPages && !cache.current.has(keyOf(p + 1))) {
      const nk = keyOf(p + 1)
      words
        .list(bookId, p + 1, size, {
          ...(seed ? { order: 'random', seed } : {}),
          ...(tagParam ? { tag: tagParam } : {}),
        })
        .then((r) => {
          cache.current.set(nk, r)
          setNeighbor((n) => ({ ...n, next: r }))
        })
        .catch(() => {})
    }
  }

  /** 从缓存同步相邻页数据（翻页后相邻页已预取或已缓存） */
  function syncNeighbor(p: number, size: number, seed: string | null, tagIds: number[]) {
    const keyOf = (pp: number) => `${bookId}:${pp}:${size}:${seed ?? ''}:${tagIds.join(',')}`
    setNeighbor({
      prev: cache.current.get(keyOf(p - 1)) ?? null,
      next: cache.current.get(keyOf(p + 1)) ?? null,
    })
  }

  /** 打乱 / 恢复顺序：换 seed 并清缓存重载第 1 页（作用于当前标签筛选后的集合）；遮挡状态按词 id 存储，与顺序无关，保持 */
  function toggleShuffle() {
    const seed = shuffleSeed ? null : String(Date.now())
    setShuffleSeed(seed)
    cache.current.clear()
    setPage(1)
    loadPage(1, pageSize, { prefetch: true, seed, tagIds: filterTagIds })
  }

  /** 重新打乱：保持打乱开启，换新 seed（仅打乱状态下可点）；遮挡状态保持 */
  function reshuffle() {
    if (!shuffleSeed) return
    const seed = String(Date.now())
    setShuffleSeed(seed)
    cache.current.clear()
    setPage(1)
    loadPage(1, pageSize, { prefetch: true, seed, tagIds: filterTagIds })
  }

  /** 标签筛选变更：更新状态并清缓存重载第 1 页（打乱状态保持，重新作用于新集合） */
  function changeTagFilter(next: number[]) {
    setFilterTagIds(next)
    cache.current.clear()
    setPage(1)
    loadPage(1, pageSize, { prefetch: true, seed: shuffleSeed, tagIds: next })
  }

  function toggleTagFilter(id: number) {
    if (filterTagIds.includes(id)) {
      changeTagFilter(filterTagIds.filter((x) => x !== id))
    } else {
      changeTagFilter([...filterTagIds, id])
    }
  }

  /** 标签管理弹窗变更后：刷新标签列表、剔除已删除标签的筛选、重载数据 */
  async function handleTagsChanged() {
    let fresh: Tag[] = []
    try {
      fresh = await tagApi.list(bookId)
      setTags(fresh)
    } catch {
      // 标签刷新失败不阻塞列表重载
    }
    const valid = new Set(fresh.map((t) => t.id))
    const next = filterTagIds.filter((id) => valid.has(id))
    if (next.length !== filterTagIds.length) {
      setFilterTagIds(next)
      cache.current.clear()
      setPage(1)
      loadPage(1, pageSize, { prefetch: true, seed: shuffleSeed, tagIds: next })
    } else {
      cache.current.clear()
      loadPage(page, pageSize, { prefetch: true })
    }
    // 列表行内标签 chips 与词数变化：强制列表模式重新查询
    setListRefresh((k) => k + 1)
  }

  // 首次加载（含预取）：从持久化页码开始，越界由 loadPage 空页回退修正
  useEffect(() => {
    loadPage(page, pageSize, { prefetch: true })
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

  /** 四个全局动作：总是清空手动例外，结果确定、与操作顺序无关（无优先级） */
  function handleHideAllWord() {
    setCover((s) => ({ ...s, hideAllWord: true, wordDiff: {} }))
  }
  function handleShowAllWord() {
    setCover((s) => ({ ...s, hideAllWord: false, wordDiff: {} }))
  }
  function handleHideAllDef() {
    setCover((s) => ({ ...s, hideAllDef: true, defDiff: {} }))
  }
  function handleShowAllDef() {
    setCover((s) => ({ ...s, hideAllDef: false, defDiff: {} }))
  }

  /** 手动点击单词/释义：以当前实际显示状态翻转，写死该词绝对状态。`in` 判定（不用 ??，diff 值为 false 时 ?? 会错误落到基线）；useCallback 保持引用稳定（Sheet memo） */
  const toggleWord = useCallback((id: number) => {
    setCover((s) => ({
      ...s,
      wordDiff: { ...s.wordDiff, [id]: !(id in s.wordDiff ? s.wordDiff[id] : s.hideAllWord) },
    }))
  }, [])
  const toggleDef = useCallback((id: number) => {
    setCover((s) => ({
      ...s,
      defDiff: { ...s.defDiff, [id]: !(id in s.defDiff ? s.defDiff[id] : s.hideAllDef) },
    }))
  }, [])

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
    // 标签词数可能变化
    try {
      setTags(await tagApi.list(bookId))
    } catch {
      // 标签刷新失败不影响主流程
    }
  }

  /** 纸质书 ↔ 列表切换：View Transitions 交叉淡化（与页面切换一致）；不支持时直接切换 */
  function changeMode(m: 'paper' | 'list') {
    if (m === mode) return
    const apply = () => setMode(m)
    if (document.startViewTransition) {
      document.startViewTransition(() => flushSync(apply))
    } else {
      apply()
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

  function handleDelete(w: Word) {
    setPendingDelete(w)
  }

  async function confirmDelete() {
    if (!pendingDelete) return
    const w = pendingDelete
    setPendingDelete(null)
    try {
      await words.remove(bookId, w.id)
      await onMutated()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    }
  }

  const actBtn =
    'inline-flex items-center px-3 py-1.5 rounded-full text-sm font-medium transition-colors duration-200 whitespace-nowrap text-charcoal/70 hover:text-charcoal hover:bg-sand/40 active:bg-sand/70'
  // 恢复按钮按需出现：存在任何手动/基线隐藏时才有“显示”意义，平时导航栏只留两个隐藏按钮
  const hasWordOverrides = cover.hideAllWord || Object.keys(cover.wordDiff).length > 0
  const hasDefOverrides = cover.hideAllDef || Object.keys(cover.defDiff).length > 0

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
                    onClick={() => changeMode(m)}
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
              {/* 标签筛选（两种模式共用；纸质书模式下打乱作用于筛选后的集合） */}
              <div className="shrink-0">
                <button
                  onClick={() => setTagFilterOpen((o) => !o)}
                  aria-pressed={filterTagIds.length > 0}
                  className={`inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap ${
                    filterTagIds.length > 0
                      ? 'bg-charcoal text-ivory shadow-md'
                      : 'text-charcoal/70 hover:text-charcoal'
                  }`}
                >
                  <TagIcon className="w-4 h-4" />
                  {filterTagIds.length > 0 ? `标签(${filterTagIds.length})` : '标签'}
                </button>
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
              {/* 隐藏/显示（纸质书模式）：四个明确动作，任何操作都是绝对设置（无优先级）；打乱时重置 */}
              {mode === 'paper' && (
                <>
                <div className="bg-sand/30 rounded-full p-1 flex shrink-0" role="group" aria-label="隐藏与显示">
                  <button onClick={handleHideAllWord} title="隐藏全部单词（含音标）" className={actBtn}>
                    隐藏单词
                  </button>
                  {/* 显示按钮按需出现：minmax(0,1fr)↔minmax(0,0fr) 轨道插值 + opacity；fr 轨道必须显式 minmax(0) 才能塌缩 */}
                  <span
                    aria-hidden={!hasWordOverrides}
                    className={`grid transition-[grid-template-columns,opacity] duration-300 ${
                      hasWordOverrides
                        ? 'grid-cols-[minmax(0,1fr)] opacity-100'
                        : 'grid-cols-[minmax(0,0fr)] opacity-0 pointer-events-none'
                    }`}
                  >
                    <button
                      onClick={handleShowAllWord}
                      title="显示所有单词"
                      tabIndex={hasWordOverrides ? 0 : -1}
                      className={`${actBtn} overflow-hidden whitespace-nowrap`}
                    >
                      显示单词
                    </button>
                  </span>
                  <span aria-hidden="true" className="w-px self-stretch mx-0.5 bg-charcoal/10" />
                  <button onClick={handleHideAllDef} title="隐藏全部释义" className={actBtn}>
                    隐藏释义
                  </button>
                  {/* 显示释义：同上 */}
                  <span
                    aria-hidden={!hasDefOverrides}
                    className={`grid transition-[grid-template-columns,opacity] duration-300 ${
                      hasDefOverrides
                        ? 'grid-cols-[minmax(0,1fr)] opacity-100'
                        : 'grid-cols-[minmax(0,0fr)] opacity-0 pointer-events-none'
                    }`}
                  >
                    <button
                      onClick={handleShowAllDef}
                      title="显示所有释义"
                      tabIndex={hasDefOverrides ? 0 : -1}
                      className={`${actBtn} overflow-hidden whitespace-nowrap`}
                    >
                      显示释义
                    </button>
                  </span>
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
                {/* 重新打乱：保持打乱开启，换新随机顺序；打乱状态切换时带显隐过渡 */}
                <span
                  aria-hidden={shuffleSeed === null}
                  className={`grid transition-[grid-template-columns,opacity] duration-300 ${
                    shuffleSeed !== null
                      ? 'grid-cols-[minmax(0,1fr)] opacity-100'
                      : 'grid-cols-[minmax(0,0fr)] opacity-0 pointer-events-none'
                  }`}
                >
                  <button
                    onClick={reshuffle}
                    title="重新生成随机顺序"
                    tabIndex={shuffleSeed !== null ? 0 : -1}
                    className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap shrink-0 text-charcoal/70 hover:text-charcoal hover:bg-sand/40 overflow-hidden"
                  >
                    重新打乱
                  </button>
                </span>
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
          {/* 标签筛选面板：渲染在滚动容器外（overflow 会裁剪绝对定位），右上角对齐胶囊 */}
          {tagFilterOpen && (
            <div className="absolute right-0 top-12 z-50 w-72 max-h-96 overflow-y-auto bg-white rounded-2xl border border-charcoal/10 shadow-xl shadow-charcoal/10 p-4 animate-fade-in-up">
              <div className="flex items-center justify-between">
                <p className="font-serif text-base text-charcoal">按标签筛选</p>
                <button
                  onClick={() => setTagFilterOpen(false)}
                  className="w-7 h-7 rounded-full text-charcoal/40 hover:bg-sand/40 hover:text-charcoal transition-colors"
                  aria-label="关闭筛选"
                >
                  ✕
                </button>
              </div>
              <div className="mt-3 space-y-1">
                <button
                  onClick={() => changeTagFilter([])}
                  aria-pressed={filterTagIds.length === 0}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-xl text-sm transition-colors ${
                    filterTagIds.length === 0
                      ? 'bg-charcoal text-ivory'
                      : 'text-charcoal/70 hover:bg-sand/40'
                  }`}
                >
                  <span>全部单词</span>
                </button>
                {tags.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => toggleTagFilter(t.id)}
                    aria-pressed={filterTagIds.includes(t.id)}
                    className={`w-full flex items-center justify-between px-3 py-2 rounded-xl text-sm transition-colors ${
                      filterTagIds.includes(t.id)
                        ? 'bg-charcoal text-ivory'
                        : 'text-charcoal/70 hover:bg-sand/40'
                    }`}
                  >
                    <span className="truncate">{t.name}</span>
                    <span
                      className={`tabular-nums text-xs ${
                        filterTagIds.includes(t.id) ? 'text-ivory/60' : 'text-charcoal/40'
                      }`}
                    >
                      {t.word_count}
                    </span>
                  </button>
                ))}
              </div>
            </div>
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
              hideAllWord={cover.hideAllWord}
              hideAllDef={cover.hideAllDef}
              wordDiff={cover.wordDiff}
              defDiff={cover.defDiff}
              onToggleWord={toggleWord}
              onToggleDef={toggleDef}
              prevPage={neighbor.prev}
              nextPage={neighbor.next}
              tags={tags}
              onTagsUpdated={onMutated}
              onTagsCreated={(tag) => setTags((prev) => [...prev, tag])}
            />
          </div>
        ) : (
          <div key="list" className="animate-fade-in-up">
            <WordTable
              bookId={bookId}
              refreshKey={listRefresh}
              onEdit={handleOpenEdit}
              onDelete={handleDelete}
              onMutated={onMutated}
              tags={tags}
              tagIds={filterTagIds}
              onManageTags={() => setManageTagsOpen(true)}
              onTagsCreated={(tag) => setTags((prev) => [...prev, tag])}
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
          tags={tags}
          onTagsCreated={(tag) => setTags((prev) => [...prev, tag])}
        />
      )}

      {manageTagsOpen && (
        <TagManageModal
          bookId={bookId}
          tags={tags}
          onClose={() => setManageTagsOpen(false)}
          onChanged={handleTagsChanged}
        />
      )}

      {pendingDelete && (
        <ConfirmModal
          title={`删除「${pendingDelete.spelling}」？`}
          message="删除后无法恢复。"
          onConfirm={confirmDelete}
          onCancel={() => setPendingDelete(null)}
        />
      )}
    </div>
  )
}
