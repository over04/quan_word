import { useEffect, useMemo, useRef, useState } from 'react'
import { tags as tagApi, type Tag } from '../api'
import { useDiscardGuard, useUnsavedChanges } from '../hooks/useUnsavedChanges'
import DiscardConfirmModal from './DiscardConfirmModal'
import Modal from './Modal'
import {
  ArrowLeftIcon,
  CheckIcon,
  CloseIcon,
  PencilIcon,
  PlusIcon,
  RefreshIcon,
  TrashIcon,
} from './Icons'

interface Props {
  bookId: number
  tags: Tag[]
  onClose: () => void
  /** 保存成功后回调（父级刷新标签与列表） */
  onChanged: () => void
}

/** 标签草稿行：id=null 表示待创建；deleted 表示待删除 */
interface DraftTag {
  key: string
  id: number | null
  name: string
  originalName: string
  wordCount: number
  deleted: boolean
}

type ChangeKind = 'add' | 'remove' | 'rename'

/** 右栏「待保存改动」的一条：文本 + 撤销后焦点落点 */
interface DraftChange {
  key: string
  /** 对应草稿行的 key（撤销后把焦点还给该行的动作按钮） */
  draftKey: string
  kind: ChangeKind
  text: string
  aria: string
}

const KIND_LABEL: Record<ChangeKind, string> = { add: '新增', remove: '删除', rename: '重命名' }
const KIND_ORDER: Record<ChangeKind, number> = { add: 0, remove: 1, rename: 2 }
/** countFlip 时长：与 tag-page.css 的 0.2s 对齐（柔和过渡 150–260ms） */
const FLIP_MS = 200
/** 保存成功后「已保存 HH:MM」的停留：够看清落款再关页，不拖慢关闭 */
const SAVED_HOLD_MS = 260

/** 图标按钮：圆形 44 命中区（触控标准），hover 只换底色与文字色 */
const ICON_BTN =
  'inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-charcoal/40 transition-colors duration-200 hover:bg-sand/40 hover:text-charcoal active:scale-95 disabled:text-charcoal/40'

/** 徽章：项目统一写法（sand 底 + sand 描边 + 小号粗体宽字距） */
const BADGE =
  'inline-flex shrink-0 items-center rounded-full border border-sand bg-sand/30 px-2 py-0.5 text-xs font-bold tracking-wide text-charcoal/70'

function toDraft(t: Tag): DraftTag {
  return {
    key: `id:${t.id}`,
    id: t.id,
    name: t.name,
    originalName: t.name,
    wordCount: t.word_count,
    deleted: false,
  }
}

function clockLabel(): string {
  const d = new Date()
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

function pad2(n: number): string {
  return n < 10 ? `0${n}` : String(n)
}

function reducedMotion(): boolean {
  return typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

/**
 * 管理当前单词书的全部标签：新建 / 行内重命名 / 删除均为本地草稿，点保存才提交。
 *
 * 版面（Modal 的 page 形态 = bg-ivory + 环境底图）：
 *   顶条（返回 44 + 衬线标题 + 未保存计数徽章）→ 居中纸卡（撑满剩余视口）→ 纸卡内的贴底操作条。
 * 纸卡 = #FDFAF4 / rounded-2xl / shadow-sm / border-charcoal/5，下层垫一张 #F2EAE0
 * 位移 8px 模拟纸堆，四周留 16–24px 让环境底图透出（不是贴满视口的直角纸）。
 * 纸卡高度由视口决定（不是由内容决定）：内容少时下方的空白在卡片内部，页面背景不会被露出来。
 * 纸卡内两栏：标签区（新建 + 列表卡）+ 右栏「待保存改动」卡，两栏同高；<768 塌成单列，
 * 右栏落到列表下方按 2:1 分高。列表卡与右栏清单各自滚动（6px 胶囊滚动条），
 * 取消/保存钉在纸卡底部内侧，滚动不推挤它；保存成功（已保存 HH:MM）与失败文案都在卡片内显示。
 * 列表是白卡 + divide-y，行 hover 浅沙底；删除走行内二次确认，该行右侧换成「确认 / 取消」
 * 两个 44 目标，不弹层。
 */
export default function TagManageModal({ bookId, tags: all, onClose, onChanged }: Props) {
  const [draft, setDraft] = useState<DraftTag[]>(() => all.map(toDraft))
  const [newName, setNewName] = useState('')
  const [renamingKey, setRenamingKey] = useState<string | null>(null)
  const [renameDraft, setRenameDraft] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  /** 行内二次确认：当前停在「确认 / 取消」的那一行 */
  const [confirmKey, setConfirmKey] = useState<string | null>(null)
  /** 保存成功的落款时刻（HH:MM），非空即处于「已保存」停留期 */
  const [savedAt, setSavedAt] = useState('')
  const [seq, setSeq] = useState(0)
  /** countFlip：正在从旧计数翻到新计数（null = 静止） */
  const [flipFrom, setFlipFrom] = useState<number | null>(null)
  const lastTotal = useRef<number | null>(null)

  const listRef = useRef<HTMLDivElement>(null)
  const newInputRef = useRef<HTMLInputElement>(null)
  const timers = useRef<Set<number>>(new Set())

  useEffect(() => {
    const pending = timers.current
    return () => {
      pending.forEach((id) => window.clearTimeout(id))
      pending.clear()
    }
  }, [])

  const dirty = useMemo(
    () => draft.some((d) => d.deleted || d.id === null || d.name !== d.originalName),
    [draft],
  )
  const aliveCount = useMemo(() => draft.filter((d) => !d.deleted).length, [draft])
  const addedCount = useMemo(() => draft.filter((d) => d.id === null && !d.deleted).length, [draft])
  const removedCount = useMemo(() => draft.filter((d) => d.id !== null && d.deleted).length, [draft])
  const renamedCount = useMemo(
    () => draft.filter((d) => !d.deleted && d.id !== null && d.name !== d.originalName).length,
    [draft],
  )
  const totalChanges = addedCount + removedCount + renamedCount

  /** 右栏清单：草稿逐条派生（新增 / 删除 / 重命名），按类别归堆 */
  const changes = useMemo<DraftChange[]>(() => {
    const out: DraftChange[] = []
    for (const d of draft) {
      if (d.deleted) {
        if (d.id !== null) {
          out.push({
            key: `remove:${d.key}`,
            draftKey: d.key,
            kind: 'remove',
            text: d.name,
            aria: `撤销删除「${d.name}」`,
          })
        }
        continue
      }
      if (d.id === null) {
        out.push({
          key: `add:${d.key}`,
          draftKey: d.key,
          kind: 'add',
          text: d.name,
          aria: `撤销新增「${d.name}」`,
        })
        continue
      }
      if (d.name !== d.originalName) {
        out.push({
          key: `rename:${d.key}`,
          draftKey: d.key,
          kind: 'rename',
          text: `${d.originalName} → ${d.name}`,
          aria: `撤销重命名「${d.originalName}」`,
        })
      }
    }
    return out.sort((a, b) => KIND_ORDER[a.kind] - KIND_ORDER[b.kind])
  }, [draft])

  const guard = useDiscardGuard(dirty, onClose)
  useUnsavedChanges(dirty)

  // Ctrl/⌘+S：挂在 window 上，焦点在页面任意位置都能保存（原来标签页完全没有快捷键）
  const saveRef = useRef<() => void>(() => {})
  saveRef.current = () => handleSave()
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault()
        saveRef.current()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // 草稿计数变化：旧数字向上翻出、新数字从下方落位（0.2s，只碰 transform）
  useEffect(() => {
    const prev = lastTotal.current
    lastTotal.current = totalChanges
    if (prev === null || prev === totalChanges) return
    if (reducedMotion()) {
      setFlipFrom(null)
      return
    }
    setFlipFrom(prev)
    const id = window.setTimeout(() => {
      timers.current.delete(id)
      setFlipFrom(null)
    }, FLIP_MS)
    timers.current.add(id)
    return () => {
      timers.current.delete(id)
      window.clearTimeout(id)
    }
  }, [totalChanges])

  function requestClose() {
    if (busy) return
    if (savedAt) {
      onClose()
      return
    }
    guard.requestClose()
  }

  /** 行内按钮在状态切换后会重挂载：下一帧把焦点还给对应动作，键盘不丢位 */
  function focusAction(key: string, act: string) {
    const root = listRef.current
    if (!root) return
    const hit = Array.from(root.querySelectorAll<HTMLElement>('[data-act]')).find(
      (node) => node.dataset.act === act && node.dataset.key === key,
    )
    hit?.focus({ preventScroll: true })
  }

  function focusAfterRender(key: string, act: string) {
    window.requestAnimationFrame(() => focusAction(key, act))
  }

  /** 新建行追加在列表末尾：列表是自身滚动区，必须把新行带进视野（不抢焦点） */
  function revealListBottom() {
    window.requestAnimationFrame(() => {
      const root = listRef.current
      if (!root) return
      root.scrollTo({ top: root.scrollHeight, behavior: reducedMotion() ? 'auto' : 'smooth' })
    })
  }

  /** 行被抽走后焦点会落回 body：只有在没人接管焦点时才还给新建字段 */
  function focusNewInput() {
    window.requestAnimationFrame(() => {
      const active = document.activeElement
      if (!active || active === document.body) newInputRef.current?.focus({ preventScroll: true })
    })
  }

  function addDraft() {
    const name = newName.trim()
    if (!name) return
    if (name.length > 20) {
      setError('标签名不能超过 20 个字符')
      return
    }
    if (draft.some((d) => !d.deleted && d.name.trim().toLowerCase() === name.toLowerCase())) {
      setError(`标签「${name}」已存在`)
      return
    }
    const next = seq + 1
    setSeq(next)
    setDraft((ds) => [
      ...ds,
      {
        key: `new:${next}`,
        id: null,
        name,
        originalName: '',
        wordCount: 0,
        deleted: false,
      },
    ])
    setNewName('')
    setError('')
    revealListBottom()
  }

  function startRename(d: DraftTag) {
    if (busy) return
    setConfirmKey(null)
    setRenamingKey(d.key)
    setRenameDraft(d.name)
    setError('')
  }

  function commitRename(d: DraftTag) {
    const name = renameDraft.trim()
    if (!name) {
      setError('标签名不能为空')
      return
    }
    if (name.length > 20) {
      setError('标签名不能超过 20 个字符')
      return
    }
    if (
      draft.some(
        (x) => x.key !== d.key && !x.deleted && x.name.trim().toLowerCase() === name.toLowerCase(),
      )
    ) {
      setError(`标签「${name}」已存在`)
      return
    }
    setDraft((ds) => ds.map((x) => (x.key === d.key ? { ...x, name } : x)))
    setRenamingKey(null)
    setError('')
    focusAfterRender(d.key, 'rename')
  }

  function cancelRename(d: DraftTag) {
    setRenamingKey(null)
    focusAfterRender(d.key, 'rename')
  }

  /** 删除第一步：该行右侧换成「确认 / 取消」，焦点落到确认上 */
  function armDelete(d: DraftTag) {
    if (busy) return
    setRenamingKey(null)
    setConfirmKey(d.key)
    setError('')
    focusAfterRender(d.key, 'confirm')
  }

  function cancelDelete(d: DraftTag) {
    setConfirmKey(null)
    focusAfterRender(d.key, 'delete')
  }

  /** 删除第二步（行内确认）：待创建行直接抽走，已存标签转为待删除草稿 */
  function confirmDelete(d: DraftTag) {
    setConfirmKey(null)
    if (d.id === null) {
      setDraft((ds) => ds.filter((x) => x.key !== d.key))
      focusNewInput()
      return
    }
    setDraft((ds) => ds.map((x) => (x.key === d.key ? { ...x, deleted: true } : x)))
    focusAfterRender(d.key, 'undo')
  }

  /** 撤销待删除：草稿行回到原样（保存时才真正删除） */
  function undoDelete(d: DraftTag) {
    setDraft((ds) => ds.map((x) => (x.key === d.key ? { ...x, deleted: false } : x)))
    focusAfterRender(d.key, 'delete')
  }

  /** 右栏撤销：只回滚清单里的这一条，不牵连其它改动 */
  function undoChange(c: DraftChange) {
    if (busy) return
    if (c.kind === 'add') {
      setDraft((ds) => ds.filter((x) => x.key !== c.draftKey))
      focusNewInput()
      return
    }
    if (c.kind === 'remove') {
      setDraft((ds) => ds.map((x) => (x.key === c.draftKey ? { ...x, deleted: false } : x)))
      focusAfterRender(c.draftKey, 'delete')
      return
    }
    if (renamingKey === c.draftKey) setRenamingKey(null)
    setDraft((ds) => ds.map((x) => (x.key === c.draftKey ? { ...x, name: x.originalName } : x)))
    focusAfterRender(c.draftKey, 'rename')
  }

  function validate(): string {
    const alive = draft.filter((d) => !d.deleted)
    const seen = new Set<string>()
    for (const d of alive) {
      const name = d.name.trim()
      if (!name) return '标签名不能为空'
      if (name.length > 20) return '标签名不能超过 20 个字符'
      const lower = name.toLowerCase()
      if (seen.has(lower)) return `标签名重复：${name}`
      seen.add(lower)
    }
    return ''
  }

  function handleSave() {
    if (busy) return
    const msg = validate()
    if (msg) {
      setError(msg)
      return
    }
    void applyChanges()
  }

  async function applyChanges() {
    setBusy(true)
    setError('')
    try {
      // 顺序：删除 → 重命名 → 新建（先删除可避免重命名撞名）
      for (const d of draft) {
        if (d.deleted && d.id !== null) await tagApi.remove(bookId, d.id)
      }
      for (const d of draft) {
        if (!d.deleted && d.id !== null && d.name.trim() !== d.originalName) {
          await tagApi.update(bookId, d.id, { name: d.name.trim() })
        }
      }
      for (const d of draft) {
        if (!d.deleted && d.id === null) await tagApi.create(bookId, { name: d.name.trim() })
      }
      onChanged()
      // 先把「保存」换成「已保存 HH:MM」再关页，否则落款没有观众
      setSavedAt(clockLabel())
      const id = window.setTimeout(() => {
        timers.current.delete(id)
        onClose()
      }, SAVED_HOLD_MS)
      timers.current.add(id)
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
      setBusy(false)
      // 部分操作可能已生效：重新拉取服务端状态，避免前端与后端不一致
      try {
        const fresh = await tagApi.list(bookId)
        setDraft(fresh.map(toDraft))
        onChanged()
      } catch {
        // 刷新失败时保留草稿，用户可重试
      }
    }
  }

  const saveLabel = savedAt ? `已保存 ${savedAt}` : busy ? '保存中…' : '保存'

  const deltaText = savedAt
    ? `已保存 ${savedAt}`
    : totalChanges > 0
      ? `未保存改动 ${totalChanges} 处：新增 ${addedCount}，删除 ${removedCount}，重命名 ${renamedCount}`
      : '没有未保存的改动'

  const newOver = newName.trim().length > 20

  return (
    <Modal onClose={requestClose} page>
      <div className="tag-page">
        {/* 顶条：返回 44 + 衬线标题 + 未保存计数徽章 */}
        <header className="shrink-0 pt-3">
          <div className="tag-shell">
            <div className="flex min-h-11 items-center gap-3 md:min-h-14">
              <button
                type="button"
                onClick={requestClose}
                aria-label="返回"
                className="-ml-2 inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-charcoal/70 transition-colors duration-200 hover:bg-sand/40 hover:text-charcoal active:scale-95"
              >
                <ArrowLeftIcon className="h-5 w-5" />
              </button>
              <h2 className="min-w-0 flex-1 truncate font-serif text-3xl text-charcoal md:text-4xl">
                管理标签
              </h2>
              {/* 徽章只在「有改动 / 刚保存」时出现：干净状态下不渲染，避免一句恒定的噪声 */}
              {(savedAt || totalChanges > 0) && (
              <span
                className="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-sand bg-sand/30 px-3 py-1.5 text-xs font-bold tracking-wide text-charcoal/70"
                role="status"
                aria-live="polite"
              >
                {savedAt ? (
                  <span aria-hidden="true">已保存 {savedAt}</span>
                ) : totalChanges > 0 ? (
                  <>
                    <span
                      aria-hidden="true"
                      className="relative inline-block h-[1.15em] min-w-[2ch] overflow-hidden text-right align-bottom tabular-nums"
                    >
                      {flipFrom !== null && (
                        <span className="tag-count-old absolute inset-y-0 right-0 inline-block text-charcoal">
                          {pad2(flipFrom)}
                        </span>
                      )}
                      <span
                        key={totalChanges}
                        className={`inline-block text-charcoal ${flipFrom !== null ? 'tag-count-new' : ''}`}
                      >
                        {pad2(totalChanges)}
                      </span>
                    </span>
                    <span aria-hidden="true">处未保存</span>
                  </>
                ) : null}
                <span className="tag-sr">{deltaText}</span>
              </span>
              )}
            </div>
          </div>
        </header>

        <main className="tag-main">
          <div className="tag-shell">
            {/* 纸卡：高度撑满顶条与视口底之间的空间（四周 16–24px 让环境底图透出），
                下层垫一张 #F2EAE0（.tag-paper::before）随之同高 */}
            <div className="tag-paper animate-fade-in-up">
              <div className="tag-card relative rounded-2xl border border-charcoal/5 bg-[#FDFAF4] p-5 shadow-sm md:p-7">
                <div className="tag-sheet">
                  <div className="flex min-h-0 min-w-0 flex-col gap-6">
                    {/* 新建标签：固定高度，列表滚动时不动 */}
                    <section className="shrink-0">
                      <h3 className="text-[11px] font-bold tracking-[0.02em] text-charcoal/45">
                        新建标签
                      </h3>
                      <div className="mt-3 flex items-center gap-2">
                        <input
                          ref={newInputRef}
                          value={newName}
                          onChange={(e) => setNewName(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === 'Enter') {
                              e.preventDefault()
                              addDraft()
                            }
                          }}
                          placeholder="新标签名"
                          aria-label="新标签名"
                          disabled={busy}
                          className="min-h-11 min-w-0 flex-1 rounded-xl border border-charcoal/15 bg-white px-3.5 text-sm text-charcoal transition-colors duration-200 placeholder:text-charcoal/40 focus:border-clay"
                        />
                        <span
                          aria-hidden="true"
                          className={`shrink-0 text-xs tabular-nums ${
                            newOver ? 'font-semibold text-clay' : 'text-charcoal/40'
                          }`}
                        >
                          {newName.trim().length}/20
                        </span>
                        <span className="tag-sr">标签名最多 20 个字</span>
                        <button
                          type="button"
                          onClick={addDraft}
                          disabled={busy || !newName.trim()}
                          className="inline-flex min-h-11 shrink-0 items-center justify-center gap-1.5 rounded-full bg-charcoal px-5 text-sm font-medium text-ivory shadow-lg shadow-charcoal/10 transition-all duration-200 hover:bg-charcoal/90 active:scale-95 disabled:bg-charcoal/20 disabled:text-charcoal/40 disabled:shadow-none"
                        >
                          <PlusIcon className="h-4 w-4" />
                          新建
                        </button>
                      </div>
                    </section>

                    {/* 标签列表：白卡 + divide-y，行 hover 浅沙底。
                        卡片自身滚动（6px 胶囊滚动条），行数多少都不改变纸卡高度 */}
                    <section className="flex min-h-0 flex-1 flex-col">
                      <div className="flex shrink-0 items-baseline justify-between gap-3">
                        <h3 className="text-[11px] font-bold tracking-[0.02em] text-charcoal/45">
                          标签
                        </h3>
                        <span className="text-xs tabular-nums text-charcoal/40">
                          {aliveCount} 个
                        </span>
                      </div>
                      <div
                        ref={listRef}
                        className="tag-scroll mt-3 flex min-h-0 flex-1 flex-col overflow-y-auto rounded-2xl border border-charcoal/5 bg-white shadow-sm"
                      >
                        {draft.length === 0 ? (
                          <p className="flex flex-1 items-center justify-center px-5 py-10 text-center text-sm text-charcoal/40">
                            还没有标签
                          </p>
                        ) : (
                          <ul className="shrink-0 divide-y divide-charcoal/5">
                            {draft.map((t) => {
                              const isRenaming = renamingKey === t.key
                              const isConfirm = confirmKey === t.key
                              const isDraft = t.id === null
                              return (
                                <li
                                  key={t.key}
                                  className={`flex min-h-14 items-center gap-2 px-3 transition-colors duration-200 md:px-4 ${
                                    t.deleted ? '' : 'hover:bg-sand/20'
                                  }`}
                                >
                                  {/* 草稿行左侧 clay 圆点；已存行留空占位，名称左缘对齐 */}
                                  <span
                                    aria-hidden="true"
                                    className={`h-2 w-2 shrink-0 rounded-full ${
                                      isDraft && !t.deleted ? 'bg-clay' : 'bg-transparent'
                                    }`}
                                  />
                                  {isRenaming ? (
                                    <>
                                      <input
                                        value={renameDraft}
                                        onChange={(e) => setRenameDraft(e.target.value)}
                                        onKeyDown={(e) => {
                                          if (e.key === 'Enter') {
                                            e.preventDefault()
                                            commitRename(t)
                                          }
                                          if (e.key === 'Escape') {
                                            // 只退出重命名：不截断的话事件会冒到 Modal 的
                                            // window keydown，把整个标签页关掉
                                            e.preventDefault()
                                            e.stopPropagation()
                                            setRenamingKey(null)
                                            focusAfterRender(t.key, 'rename')
                                          }
                                        }}
                                        autoFocus
                                        aria-label={`重命名 ${t.name}`}
                                        className="min-h-11 min-w-0 flex-1 rounded-xl border border-charcoal/15 bg-white px-3 text-sm text-charcoal transition-colors duration-200 focus:border-clay"
                                      />
                                      <button
                                        type="button"
                                        onClick={() => commitRename(t)}
                                        disabled={busy}
                                        data-act="commit"
                                        data-key={t.key}
                                        aria-label="保存重命名"
                                        className={ICON_BTN}
                                      >
                                        <CheckIcon className="h-4 w-4" />
                                      </button>
                                      <button
                                        type="button"
                                        onClick={() => cancelRename(t)}
                                        disabled={busy}
                                        data-act="cancel-rename"
                                        data-key={t.key}
                                        aria-label="取消重命名"
                                        className={ICON_BTN}
                                      >
                                        <CloseIcon className="h-4 w-4" />
                                      </button>
                                    </>
                                  ) : isConfirm ? (
                                    <>
                                      <span
                                        className="min-w-0 flex-1 truncate text-sm text-charcoal"
                                        title={t.name}
                                      >
                                        {t.name}
                                      </span>
                                      <button
                                        type="button"
                                        onClick={() => confirmDelete(t)}
                                        disabled={busy}
                                        data-act="confirm"
                                        data-key={t.key}
                                        aria-label={`确认删除 ${t.name}`}
                                        className="inline-flex min-h-11 shrink-0 items-center justify-center rounded-full border border-charcoal/20 px-4 text-xs font-medium text-charcoal transition-all duration-200 hover:border-charcoal/40 hover:bg-white active:scale-95"
                                      >
                                        确认
                                      </button>
                                      <button
                                        type="button"
                                        onClick={() => cancelDelete(t)}
                                        disabled={busy}
                                        data-act="cancel-delete"
                                        data-key={t.key}
                                        aria-label="取消删除"
                                        className="inline-flex min-h-11 shrink-0 items-center justify-center rounded-full px-3 text-xs font-medium text-charcoal/70 transition-colors duration-200 hover:bg-sand/40 hover:text-charcoal active:scale-95"
                                      >
                                        取消
                                      </button>
                                    </>
                                  ) : t.deleted ? (
                                    <>
                                      <span
                                        className="min-w-0 flex-1 truncate text-sm text-charcoal/40 line-through"
                                        title={t.name}
                                      >
                                        {t.name}
                                      </span>
                                      <span className="shrink-0 text-xs text-charcoal/40">
                                        将删除
                                      </span>
                                      <button
                                        type="button"
                                        onClick={() => undoDelete(t)}
                                        disabled={busy}
                                        data-act="undo"
                                        data-key={t.key}
                                        aria-label={`撤销删除 ${t.name}`}
                                        title="撤销删除"
                                        className={ICON_BTN}
                                      >
                                        <RefreshIcon className="h-4 w-4" />
                                      </button>
                                    </>
                                  ) : (
                                    <>
                                      <button
                                        type="button"
                                        onClick={() => startRename(t)}
                                        disabled={busy}
                                        data-act="rename"
                                        data-key={t.key}
                                        aria-label={`重命名 ${t.name}`}
                                        className="group flex min-h-11 min-w-0 flex-1 items-center gap-2 rounded-xl text-left"
                                      >
                                        <span
                                          className="min-w-0 truncate text-sm text-charcoal"
                                          title={t.name}
                                        >
                                          {t.name}
                                        </span>
                                        {isDraft && <span className={BADGE}>新建</span>}
                                        <PencilIcon className="ml-auto h-3.5 w-3.5 shrink-0 text-clay opacity-0 transition-opacity duration-200 group-hover:opacity-100 group-focus-within:opacity-100" />
                                      </button>
                                      <span className="shrink-0 text-xs tabular-nums text-charcoal/40">
                                        {isDraft ? '—' : `${t.wordCount} 词`}
                                      </span>
                                      <button
                                        type="button"
                                        onClick={() => armDelete(t)}
                                        disabled={busy}
                                        data-act="delete"
                                        data-key={t.key}
                                        aria-label={`删除 ${t.name}`}
                                        title="删除"
                                        className="inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-charcoal/10 text-charcoal/40 transition-all duration-200 hover:border-red-300 hover:text-red-400 active:scale-95 disabled:text-charcoal/40"
                                      >
                                        <TrashIcon className="h-4 w-4" />
                                      </button>
                                    </>
                                  )}
                                </li>
                              )
                            })}
                          </ul>
                        )}
                      </div>
                    </section>
                  </div>

                  {/* 右栏：待保存改动清单（从 draft 派生，逐条可撤销）。
                      与左栏同高（网格行高拉满），清单自身滚动，空态在卡片里垂直居中偏上 */}
                  <aside
                    className="tag-aside min-w-0 overflow-hidden rounded-2xl border border-charcoal/5 bg-white shadow-sm"
                    aria-label="待保存改动清单"
                  >
                    <div className="flex shrink-0 flex-col gap-1 border-b border-charcoal/5 px-4 py-3">
                      <h3 className="text-[11px] font-bold tracking-[0.02em] text-charcoal/45">
                        待保存改动
                      </h3>
                      {changes.length > 0 && (
                        <span className="text-xs tabular-nums text-charcoal/40">
                          新增 {addedCount} · 删除 {removedCount} · 重命名 {renamedCount}
                        </span>
                      )}
                    </div>
                    {changes.length === 0 ? (
                      /* pb 让文字重心高于卡片几何中心（垂直居中偏上），不再贴在顶边 */
                      <p className="flex min-h-0 flex-1 items-center justify-center px-4 pb-10 text-center text-sm text-charcoal/40">
                        没有待保存的改动
                      </p>
                    ) : (
                      <ul className="tag-scroll min-h-0 flex-1 divide-y divide-charcoal/5 overflow-y-auto">
                        {changes.map((c) => (
                          <li
                            key={c.key}
                            className="flex min-h-11 items-center gap-2 px-3 transition-colors duration-200 hover:bg-sand/20"
                          >
                            <span className={BADGE}>{KIND_LABEL[c.kind]}</span>
                            <span
                              className="min-w-0 flex-1 truncate text-sm text-charcoal/70"
                              title={c.text}
                            >
                              {c.text}
                            </span>
                            <button
                              type="button"
                              onClick={() => undoChange(c)}
                              disabled={busy}
                              aria-label={c.aria}
                              title="撤销这项改动"
                              className={ICON_BTN}
                            >
                              <RefreshIcon className="h-4 w-4" />
                            </button>
                          </li>
                        ))}
                      </ul>
                    )}
                  </aside>
                </div>

                {/* 贴底操作条：取消（描边胶囊次按钮）+ 保存（实心胶囊主按钮）。
                    钉在纸卡底部内侧——列表/清单滚动、行数变化都不推挤它；
                    保存成功（已保存 HH:MM）与失败文案都在卡片内显示，不弹 toast。 */}
                <footer className="tag-foot">
                  <div className="flex min-h-14 items-center gap-4">
                    <div className="min-h-11 min-w-0 flex-1" role="alert">
                      {error && <p className="text-sm text-red-600">{error}</p>}
                    </div>
                    <div className="flex shrink-0 items-center gap-3 md:gap-6">
                      <button
                        type="button"
                        onClick={requestClose}
                        disabled={busy}
                        className="inline-flex min-h-11 shrink-0 items-center justify-center rounded-full border border-charcoal/20 px-6 text-sm font-medium text-charcoal transition-all duration-200 hover:border-charcoal/40 hover:bg-white active:scale-95 disabled:border-charcoal/10 disabled:text-charcoal/40"
                      >
                        取消
                      </button>
                      <button
                        type="button"
                        onClick={handleSave}
                        disabled={busy || !dirty}
                        className={`inline-flex min-h-11 shrink-0 items-center justify-center rounded-full px-7 text-sm font-medium text-ivory shadow-lg shadow-charcoal/10 transition-all duration-200 active:scale-95 ${
                          savedAt
                            ? 'bg-charcoal'
                            : 'bg-charcoal hover:bg-charcoal/90 disabled:bg-charcoal/20 disabled:text-charcoal/40 disabled:shadow-none'
                        }`}
                      >
                        {saveLabel}
                      </button>
                    </div>
                  </div>
                </footer>
              </div>
            </div>
          </div>
        </main>
      </div>

      {guard.confirmOpen && (
        <DiscardConfirmModal onDiscard={guard.confirmDiscard} onCancel={guard.cancelDiscard} />
      )}
    </Modal>
  )
}
