import { useCallback, useEffect, useRef, useState } from 'react'
import {
  sanitizeTagFilter,
  tags as tagApi,
  wordbooks,
  words,
  type Page,
  type Tag,
  type TagFilter,
  type Word,
  type Wordbook,
} from '../api'
import { withViewTransition } from '../viewTransition'
import PaperBookView from '../components/PaperBookView'
import TagFilterPanel from '../components/TagFilterPanel'
import { usePresence } from '../hooks/usePresence'
import TagManageModal from '../components/TagManageModal'
import ConfirmModal from '../components/ConfirmModal'
import WordTable from '../components/WordTable'
import WordFormModal from '../components/WordFormModal'
import SettingsPanel from '../components/SettingsPanel'
import ToastHost from '../components/Toast'
import { toast } from '../toast'
import {
  ArrowLeftIcon,
  BookIcon,
  ListIcon,
  MoreIcon,
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

/** 标签筛选（按书持久化）：组数组 + 组间连接词；旧单组结构（qw_filter_tags_ 与 qw_filter_match_ key）自动迁移后清除；损坏回退空 */
function loadTagFilter(bookId: number): TagFilter {
  const legacyKey = `qw_filter_tags_${bookId}`
  const legacyMatchKey = `qw_filter_match_${bookId}`
  try {
    const legacyMatch = localStorage.getItem(legacyMatchKey)
    const legacyRaw = localStorage.getItem(legacyKey)
    if (legacyRaw !== null || legacyMatch !== null) {
      // 旧结构迁移：单组（旧匹配模式 + 旧标签 ids），无连接词
      let ids: number[] = []
      try {
        const p = JSON.parse(legacyRaw ?? '[]')
        if (Array.isArray(p)) ids = p.filter((x) => Number.isFinite(x)).map(Number)
      } catch {
        // 旧值损坏：按空筛选迁移
      }
      const f: TagFilter =
        ids.length > 0
          ? { groups: [{ mode: legacyMatch === 'or' ? 'or' : 'and', ids }], links: [] }
          : { groups: [], links: [] }
      localStorage.removeItem(legacyKey)
      localStorage.removeItem(legacyMatchKey)
      return f
    }
    const raw = localStorage.getItem(`qw_filter_${bookId}`)
    if (!raw) return { groups: [], links: [] }
    const p = JSON.parse(raw)
    if (!p || typeof p !== 'object' || !Array.isArray(p.groups) || !Array.isArray(p.links)) {
      return { groups: [], links: [] }
    }
    const groups = (p.groups as Array<{ mode?: unknown; ids?: unknown }>)
      .filter(
        (g) =>
          g !== null &&
          (g.mode === 'and' || g.mode === 'or' || g.mode === 'none') &&
          Array.isArray(g.ids),
      )
      .map((g) =>
        g.mode === 'none'
          ? { mode: 'none' as const, ids: [] as number[] }
          : {
              mode: g.mode as 'and' | 'or',
              ids: (g.ids as unknown[]).filter((x) => Number.isFinite(x)).map(Number),
            },
      )
    return sanitizeTagFilter({
      groups,
      links: (p.links as unknown[]).filter((l) => l === 'and' || l === 'or') as (
        | 'and'
        | 'or'
      )[],
    })
  } catch {
    return { groups: [], links: [] }
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
  // 已加载纸页（纸质书滚动模式）：页码升序连续；首屏 1 页 + 后台预取，滚动接近底部追加
  const [pages, setPages] = useState<Array<{ d: Page<Word>; pageNo: number }>>([])
  const [mode, setMode] = useState<'paper' | 'list'>('paper')
  // 页码持久化：保存首张已加载纸的页码（"上次浏览位置"语义的近似），刷新从该页开始；
  // 数据未就绪（pages 空）时不写，避免挂载时把持久化页码重置为 1（首次加载 effect 靠它恢复位置）
  const startPage = pages[0]?.pageNo ?? null
  useEffect(() => {
    if (startPage === null) return
    localStorage.setItem(`qw_page_${bookId}`, String(startPage))
  }, [startPage, bookId])
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
  // 手机端「更多」溢出菜单：隐藏/显示/打乱等低频动作收进来，导航不再横向溢出
  const [moreOpen, setMoreOpen] = useState(false)
  const [error, setError] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Word | null>(null)
  // 待确认删除的词（ConfirmModal 替换原生 confirm）
  const [pendingDelete, setPendingDelete] = useState<Word | null>(null)
  // 标签：该书全部标签 + 筛选（组数组：组间且、组内 and/or，两模式共享）+ 管理弹窗；筛选按书持久化，刷新后恢复
  const [tags, setTags] = useState<Tag[]>([])
  const [tagFilter, setTagFilter] = useState<TagFilter>(() => loadTagFilter(bookId))
  useEffect(() => {
    localStorage.setItem(`qw_filter_${bookId}`, JSON.stringify(tagFilter))
  }, [tagFilter, bookId])
  const [tagFilterOpen, setTagFilterOpen] = useState(false)
  /** 三个浮层都带退场：关闭后多留 180ms 播淡出，再卸载 */
  const moreMenu = usePresence(moreOpen, 180)
  const settingsPanel = usePresence(settingsOpen, 180)
  const filterPanel = usePresence(tagFilterOpen, 180)
  const [manageTagsOpen, setManageTagsOpen] = useState(false)
  // 列表模式刷新信号：增删改后递增，WordTable 重新查询
  const [listRefresh, setListRefresh] = useState(0)

  // 分页缓存：key = `${bookId}:${page}:${size}:${seed}:${filter}`，避免重复请求
  const cache = useRef<Map<string, Page<Word>>>(new Map())
  // 请求序号：筛选/打乱/重载快速连续操作时丢弃过期响应（最后一次调用生效）
  const seqRef = useRef(0)
  // 下一页加载中（哨兵触发后，底部显示占位）
  const [loadingMore, setLoadingMore] = useState(false)
  const loadingMoreRef = useRef(false)
  // in-flight 请求去重：同一 key 并发请求复用同一 Promise
  const inflight = useRef(new Map<string, Promise<Page<Word> | null>>())

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

  /** 取一页数据：缓存命中同步返回；in-flight 同 key 并发复用同一 Promise；成功填缓存；失败 setError 并返回 null */
  async function fetchPageRaw(
    p: number,
    size: number,
    seed: string | null,
    tagFilter: TagFilter,
  ): Promise<Page<Word> | null> {
    const key = `${bookId}:${p}:${size}:${seed ?? ''}:${JSON.stringify(tagFilter)}`
    const cached = cache.current.get(key)
    if (cached) return cached
    const running = inflight.current.get(key)
    if (running) return running
    const prom = words
      .list(bookId, p, size, {
        ...(seed ? { order: 'random', seed } : {}),
        tagGroups: tagFilter,
      })
      .then((r) => {
        cache.current.set(key, r)
        return r
      })
      .catch((e) => {
        setError(e instanceof Error ? e.message : '加载失败')
        return null
      })
      .finally(() => {
        inflight.current.delete(key)
      })
    inflight.current.set(key, prom)
    return prom
  }

  /** UI 路径取页：带请求序号，过期响应（筛选/打乱/重载后）不生效 */
  async function fetchPage(
    p: number,
    size: number,
    seed: string | null,
    tagFilter: TagFilter,
  ): Promise<Page<Word> | null> {
    const seq = ++seqRef.current
    const r = await fetchPageRaw(p, size, seed, tagFilter)
    if (seq !== seqRef.current) return null // 过期响应：丢弃
    return r
  }

  /** 后台预取下一页填缓存（不 setState）；已缓存/越界跳过 */
  function prefetchNext(p: number, totalPages: number, seed: string | null, tagFilter: TagFilter, size: number) {
    if (p > totalPages) return
    const key = `${bookId}:${p}:${size}:${seed ?? ''}:${JSON.stringify(tagFilter)}`
    if (cache.current.has(key)) return
    void fetchPageRaw(p, size, seed, tagFilter)
  }

  /** 从第 p 页开始重建；不清空 pages，新数据到达才整体替换（避免加载闪白） */
  async function loadFrom(p: number, opts?: { seed?: string | null; tagFilter?: TagFilter; size?: number }) {
    const size = opts?.size ?? pageSize
    const seed = opts?.seed !== undefined ? opts.seed : shuffleSeed
    const activeFilter = opts?.tagFilter ?? tagFilter
    let target = p
    let paged = await fetchPage(target, size, seed, activeFilter)
    // 空页回退：持久化页码越界（词被删页数减少）或删除后当前页删空 → 逐页回退到有数据的页
    while (paged && paged.items.length === 0 && target > 1) {
      target -= 1
      paged = await fetchPage(target, size, seed, activeFilter)
    }
    if (!paged) return // 过期或失败：不更新 UI
    setError('')
    setPages([{ d: paged, pageNo: target }])
    prefetchNext(target + 1, paged.total_pages, seed, activeFilter, size)
    prefetchNext(target + 2, paged.total_pages, seed, activeFilter, size)
  }

  /** 哨兵回调：加载下一页并追加；loadingMoreRef 防重入；已到末页 no-op */
  const loadMore = useCallback(
    async (nextPageNo: number, totalPages: number) => {
      if (loadingMoreRef.current || nextPageNo > totalPages) return
      loadingMoreRef.current = true
      setLoadingMore(true)
      const d = await fetchPage(nextPageNo, pageSize, shuffleSeed, tagFilter)
      loadingMoreRef.current = false
      setLoadingMore(false)
      if (d && d.items.length > 0) {
        setPages((p) => [...p, { d, pageNo: nextPageNo }])
        prefetchNext(nextPageNo + 1, d.total_pages, shuffleSeed, tagFilter, pageSize)
      }
    },
    [bookId, pageSize, shuffleSeed, tagFilter],
  )

  /** 打乱 / 恢复顺序：换 seed 并清缓存重载第 1 页（作用于当前标签筛选后的集合）；遮挡状态按词 id 存储，与顺序无关，保持 */
  function toggleShuffle() {
    const seed = shuffleSeed ? null : String(Date.now())
    setShuffleSeed(seed)
    cache.current.clear()
    loadFrom(1, { seed, tagFilter })
    toast(seed ? '已打乱顺序' : '已恢复原始顺序')
  }

  /** 重新打乱：保持打乱开启，换新 seed（仅打乱状态下可点）；遮挡状态保持 */
  function reshuffle() {
    if (!shuffleSeed) return
    const seed = String(Date.now())
    setShuffleSeed(seed)
    cache.current.clear()
    loadFrom(1, { seed, tagFilter })
    toast('已重新打乱')
  }

  /** 标签筛选变更：函数式更新（连续操作按序应用）；清缓存与重载由 tagFilter effect 统一处理 */
  function applyTagFilter(updater: (prev: TagFilter) => TagFilter) {
    setTagFilter(updater)
  }

  // 筛选变更（跳过首次挂载）：清缓存重载第 1 页（打乱状态保持，重新作用于新集合）
  const firstFilterRef = useRef(true)
  useEffect(() => {
    if (firstFilterRef.current) {
      firstFilterRef.current = false
      return
    }
    cache.current.clear()
    loadFrom(1, { seed: shuffleSeed, tagFilter })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tagFilter])

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
    setTagFilter((prev) => {
      // 剔除已删除标签；净化后空组自动移除
      const next = sanitizeTagFilter({
        groups: prev.groups.map((g) => ({
          ...g,
          ids: g.ids.filter((id) => valid.has(id)),
        })),
        links: prev.links,
      })
      // 筛选未受影响：返回原引用，React bail out，不触发重载
      return JSON.stringify(next) === JSON.stringify(prev) ? prev : next
    })
    // 列表行内标签 chips 与词数变化：强制列表模式重新查询
    setListRefresh((k) => k + 1)
  }

  // 首次加载：从持久化页码开始，越界由 loadFrom 空页回退修正
  useEffect(() => {
    loadFrom(loadPageNo(bookId))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  function changePageSize(n: number) {
    localStorage.setItem('qw_page_size', String(n))
    cache.current.clear()
    setPageSize(n)
    loadFrom(1, { size: n })
  }

  function changeFontScale(s: number) {
    localStorage.setItem('qw_font_scale', String(s))
    setFontScale(s)
  }

  /** 四个全局动作：总是清空手动例外，结果确定、与操作顺序无关（无优先级） */
  function handleHideAllWord() {
    setCover((s) => ({ ...s, hideAllWord: true, wordDiff: {} }))
    toast('已隐藏全部单词')
  }
  function handleShowAllWord() {
    setCover((s) => ({ ...s, hideAllWord: false, wordDiff: {} }))
    toast('已显示全部单词')
  }
  function handleHideAllDef() {
    setCover((s) => ({ ...s, hideAllDef: true, defDiff: {} }))
    toast('已隐藏全部释义')
  }
  function handleShowAllDef() {
    setCover((s) => ({ ...s, hideAllDef: false, defDiff: {} }))
    toast('已显示全部释义')
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

  /** 增删改后：清缓存重载当前起始页 + 刷新书信息 + 触发列表模式重新查询 */
  async function onMutated() {
    cache.current.clear()
    await loadFrom(pages[0]?.pageNo ?? 1)
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

  /** 纸质书 ↔ 列表切换：走统一的 View Transition（串行化 + vt-running），避免与页面切换互相 skip */
  function changeMode(m: 'paper' | 'list') {
    if (m === mode) return
    void withViewTransition(() => setMode(m))
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
    'inline-flex items-center min-h-11 px-3 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap text-charcoal/70 hover:text-charcoal hover:bg-sand/40 active:bg-sand/70 active:scale-95'
  // 恢复按钮按需出现：存在任何手动/基线隐藏时才有“显示”意义，平时导航栏只留两个隐藏按钮
  const hasWordOverrides = cover.hideAllWord || Object.keys(cover.wordDiff).length > 0
  const hasDefOverrides = cover.hideAllDef || Object.keys(cover.defDiff).length > 0
  // 标签筛选：净化后仍有有效条件（含无标签组）即生效；徽标显示选中标签总数（递归计数）
  const hasFilter = sanitizeTagFilter(tagFilter).groups.length > 0
  const totalSelected = tagFilter.groups.reduce((n, g) => n + g.ids.length, 0)
  // 手机端溢出菜单里的纸质书动作（桌面端仍以胶囊组平铺，见下方 hidden md:flex）
  const paperActions = [
    { key: 'settings', label: '显示设置', onClick: () => setSettingsOpen(true) },
    { key: 'hideWord', label: '隐藏单词', onClick: handleHideAllWord },
    { key: 'showWord', label: '显示单词', onClick: handleShowAllWord, show: hasWordOverrides },
    { key: 'hideDef', label: '隐藏释义', onClick: handleHideAllDef },
    { key: 'showDef', label: '显示释义', onClick: handleShowAllDef, show: hasDefOverrides },
    { key: 'shuffle', label: shuffleSeed !== null ? '取消打乱' : '打乱', onClick: toggleShuffle },
    { key: 'reshuffle', label: '重新打乱', onClick: reshuffle, show: shuffleSeed !== null },
  ].filter((item) => item.show !== false)


  return (
    <div className="min-h-screen vt-book-page outline-none" data-page-root="" tabIndex={-1}>
      <ToastHost />
      {/* 悬浮胶囊导航 */}
      <nav className="vt-nav-book fixed w-full top-0 z-40 pb-2 pt-[calc(env(safe-area-inset-top,0px)+0.5rem)] md:py-4 px-4 md:px-8 animate-nav-in">
        <div className="max-w-7xl mx-auto">
          {/* 外层 relative：设置浮层锚点（在滚动容器外，避免被 overflow-x-auto 裁剪） */}
          <div className="relative">
          <div className="bg-ivory/95 border border-white/40 shadow-sm rounded-3xl md:rounded-full px-3 md:px-6 py-1.5 md:py-3 flex flex-wrap items-center gap-2 md:gap-3">
            <button
              onClick={onBack}
              className="inline-flex items-center gap-1.5 min-h-11 px-3 rounded-full text-sm font-medium text-charcoal/70 hover:bg-sand/40 hover:text-charcoal active:bg-sand/70 active:scale-95 transition-all whitespace-nowrap shrink-0"
            >
              <ArrowLeftIcon className="w-4 h-4" />
              返回
            </button>
            <div className="hidden md:flex w-7 h-7 rounded-full bg-sage items-center justify-center text-white shrink-0">
              <BookIcon className="w-3.5 h-3.5" />
            </div>
            <div className="flex items-center min-w-0 flex-1">
              <h1 className="font-serif text-base text-charcoal truncate">{book ? book.name : '…'}</h1>
            </div>
            {pages[0] && (
              <span className="hidden md:inline shrink-0 text-xs text-charcoal/45 tabular-nums whitespace-nowrap">
                {pages[0].d.total} 词
              </span>
            )}
            <button
              onClick={handleOpenCreate}
              aria-label="添加单词"
              className="ml-auto inline-flex items-center justify-center gap-1.5 min-h-11 min-w-11 bg-charcoal text-ivory px-0 md:px-4 rounded-full text-sm font-medium hover:bg-charcoal/90 active:scale-95 transition-all shadow-lg shadow-charcoal/10 whitespace-nowrap shrink-0"
            >
              <PlusIcon className="w-4 h-4" />
              <span className="hidden md:inline">添加单词</span>
            </button>
            <div className="flex w-full flex-wrap items-center gap-1.5 md:gap-2.5 md:ml-auto md:w-auto">
              <div className="bg-sand/30 rounded-full p-1 flex shrink-0" role="tablist" aria-label="视图模式">
                {(['paper', 'list'] as const).map((m) => (
                  <button
                    key={m}
                    role="tab"
                    aria-selected={mode === m}
                    onClick={() => changeMode(m)}
                    className={`inline-flex items-center gap-1.5 min-h-11 px-3.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap active:scale-95 ${
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
                  aria-pressed={hasFilter}
                  className={`inline-flex items-center gap-1.5 min-h-11 px-3.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap active:scale-95 ${
                    hasFilter
                      ? 'bg-charcoal text-ivory shadow-md'
                      : 'text-charcoal/70 hover:text-charcoal'
                  }`}
                >
                  <TagIcon className="w-4 h-4" />
                  {hasFilter ? (totalSelected > 0 ? `标签(${totalSelected})` : '标签(无标签)') : '标签'}
                </button>
              </div>
              {/* 显示设置（仅纸质书模式，位置在模式切换旁） */}
              {mode === 'paper' && (
                <button
                  onClick={() => setSettingsOpen((o) => !o)}
                  className={`hidden md:flex w-11 h-11 rounded-full items-center justify-center transition-all shrink-0 active:scale-95 ${
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
              {/* 隐藏/显示（纸质书模式）：桌面平铺；手机收进「更多」，避免横向溢出 */}
              {mode === 'paper' && (
                <div className="hidden md:flex items-center gap-2.5">
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
                  className={`inline-flex items-center gap-1.5 min-h-11 px-4 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap shrink-0 active:scale-95 ${
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
                    className="inline-flex items-center gap-1.5 min-h-11 px-4 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap shrink-0 text-charcoal/70 hover:text-charcoal hover:bg-sand/40 active:bg-sand/70 active:scale-95 overflow-hidden"
                  >
                    重新打乱
                  </button>
                </span>
                </div>
              )}
              {/* 手机端溢出菜单入口 */}
              {mode === 'paper' && (
                <button
                  type="button"
                  onClick={() => setMoreOpen((open) => !open)}
                  aria-label="更多操作"
                  aria-expanded={moreOpen}
                  aria-pressed={hasWordOverrides || hasDefOverrides || shuffleSeed !== null}
                  className={`relative shrink-0 w-11 h-11 rounded-full flex items-center justify-center transition-all active:scale-95 ${
                    moreOpen || hasWordOverrides || hasDefOverrides || shuffleSeed !== null
                      ? 'bg-charcoal text-ivory'
                      : 'bg-sand/40 text-charcoal/70 hover:bg-sand/70'
                  } md:hidden`}
                >
                  <MoreIcon className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>
          {/* 手机端溢出菜单：隐藏/显示/打乱（桌面端这些动作平铺在导航里） */}
          {moreMenu.mounted && (
            <>
              <button
                type="button"
                aria-label="关闭更多操作"
                tabIndex={-1}
                onClick={() => setMoreOpen(false)}
                className={
                  'md:hidden fixed inset-0 z-40 cursor-default' + (moreOpen ? '' : ' pointer-events-none')
                }
              />
              <div
                data-state={moreOpen ? 'open' : 'closing'}
                className="ui-pop md:hidden absolute right-0 top-full mt-2 w-56 rounded-2xl border border-charcoal/10 bg-white p-2 shadow-xl shadow-charcoal/10 z-50"
              >
                {paperActions.map((action) => (
                  <button
                    key={action.key}
                    type="button"
                    onClick={() => {
                      setMoreOpen(false)
                      action.onClick()
                    }}
                    className="w-full min-h-11 px-3 rounded-xl text-left text-sm text-charcoal/80 hover:bg-sand/40 active:bg-sand/60 active:scale-[0.99] transition-all"
                  >
                    {action.label}
                  </button>
                ))}
              </div>
            </>
          )}

          {/* 设置面板：渲染在滚动容器外，右上角对齐胶囊 */}
          {settingsPanel.mounted && (
            <SettingsPanel
              pageSize={pageSize}
              fontScale={fontScale}
              onChangePageSize={changePageSize}
              onChangeFontScale={changeFontScale}
              onClose={() => setSettingsOpen(false)}
              closing={!settingsOpen}
            />
          )}
          {/* 标签筛选面板：渲染在滚动容器外（overflow 会裁剪绝对定位），右上角对齐胶囊 */}
          {filterPanel.mounted && (
            <TagFilterPanel
              tags={tags}
              filter={tagFilter}
              onChange={applyTagFilter}
              onClose={() => setTagFilterOpen(false)}
              closing={!tagFilterOpen}
            />
          )}
          </div>
        </div>
      </nav>

      <main className="max-w-7xl mx-auto px-4 md:px-8 pt-[calc(env(safe-area-inset-top,0px)+9.5rem)] md:pt-[calc(env(safe-area-inset-top,0px)+8rem)] pb-16">
        {error && <p className="text-red-600 text-center mb-6">{error}</p>}

        {mode === 'paper' ? (
          <div key="paper" className="animate-fade-in-up">
            <PaperBookView
              pages={pages}
              loading={loadingMore}
              onReachEnd={loadMore}
              onAddFirst={handleOpenCreate}
              filtered={hasFilter}
              onClearFilter={() => applyTagFilter(() => ({ groups: [], links: [] }))}
              fontScale={fontScale}
              hideAllWord={cover.hideAllWord}
              hideAllDef={cover.hideAllDef}
              wordDiff={cover.wordDiff}
              defDiff={cover.defDiff}
              onToggleWord={toggleWord}
              onToggleDef={toggleDef}
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
              tagFilter={tagFilter}
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
