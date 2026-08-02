import { useEffect, useMemo, useRef, useState } from 'react'
import { words, type Page, type Tag, type Word } from '../api'
import ImportModal from './ImportModal'
import Pagination from './Pagination'
import TagPickerModal from './TagPickerModal'
import {
  ArrowsUpDownIcon,
  PencilIcon,
  SearchIcon,
  TagIcon,
  TrashIcon,
  UploadIcon,
} from './Icons'

interface Props {
  bookId: number
  /** 增删改后递增，触发重新加载 */
  refreshKey: number
  onEdit: (w: Word) => void
  onDelete: (w: Word) => void
  /** 导入/批量删除等内部变更后通知父级（刷新书信息与缓存） */
  onMutated: () => void
  /** 该书全部标签（行内展示与批量打标签用） */
  tags: Tag[]
  /** 当前标签筛选（多选交集） */
  tagIds: number[]
  /** 打开标签管理弹窗 */
  onManageTags: () => void
  /** 新建标签成功后回调（父级刷新标签列表） */
  onTagsCreated: (tag: Tag) => void
}

const PAGE_SIZE = 20

const SORTS: Array<{ key: string; label: string }> = [
  { key: 'created_at', label: '添加时间' },
  { key: 'spelling', label: '拼写' },
  { key: 'updated_at', label: '更新时间' },
]

function summary(w: Word): string {
  return w.definitions.map((d) => (d.pos ? `${d.pos} ${d.meaning}` : d.meaning)).join('；')
}

function formatDate(iso: string): string {
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleDateString('zh-CN')
}

/** 列表模式：书内搜索 + 排序 + 标签筛选 + 分页 + 批量管理（自包含数据加载） */
export default function WordTable({
  bookId,
  refreshKey,
  onEdit,
  onDelete,
  onMutated,
  tags,
  tagIds,
  onManageTags,
  onTagsCreated,
}: Props) {
  const [q, setQ] = useState('')
  const [debouncedQ, setDebouncedQ] = useState('')
  const [sort, setSort] = useState('created_at')
  const [order, setOrder] = useState<'asc' | 'desc'>('asc')
  const [page, setPage] = useState(1)
  const [data, setData] = useState<Page<Word> | null>(null)
  const [error, setError] = useState('')
  // 数据版本：查询结果到达时递增，驱动数据容器重挂播放淡入动画
  const [dataVersion, setDataVersion] = useState(0)
  // 批量选择：Set 跨页累计保留；删除/导入成功后清空
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [importOpen, setImportOpen] = useState(false)
  // 批量打标签弹窗
  const [tagPickerOpen, setTagPickerOpen] = useState(false)
  // 上次筛选（引用比较）：变化时重置页码且不发起旧页请求
  const prevTagRef = useRef(tagIds)
  // 标签 id → 名称映射（行内 chips）
  const tagName = useMemo(() => new Map(tags.map((t) => [t.id, t.name])), [tags])

  // 搜索防抖：停止输入 400ms 后生效
  useEffect(() => {
    const t = window.setTimeout(() => setDebouncedQ(q.trim()), 400)
    return () => window.clearTimeout(t)
  }, [q])

  // 查询加载：搜索词 / 排序 / 页码 / 标签筛选 / 数据变更时重新请求
  useEffect(() => {
    if (prevTagRef.current !== tagIds) {
      prevTagRef.current = tagIds
      if (page !== 1) {
        // 筛选变化：回到第 1 页，等待重渲染后以新筛选查询
        setPage(1)
        return
      }
      // 已在第 1 页：setPage(1) 不会触发重渲染，直接以新筛选查询
    }
    let alive = true
    words
      .query(bookId, page, PAGE_SIZE, {
        q: debouncedQ || undefined,
        sort,
        order,
        tag: tagIds.length > 0 ? tagIds.join(',') : undefined,
      })
      .then((r) => {
        if (alive) {
          setData(r)
          setDataVersion((v) => v + 1)
        }
      })
      .catch((e) => {
        if (alive) setError(e instanceof Error ? e.message : '加载失败')
      })
    return () => {
      alive = false
    }
  }, [bookId, debouncedQ, sort, order, page, refreshKey, tagIds])

  // 删除后当前页可能空：回退上一页
  useEffect(() => {
    if (data && data.items.length === 0 && page > 1) setPage((p) => p - 1)
  }, [data, page])

  function onQChange(v: string) {
    setQ(v)
    setPage(1)
  }

  const pageIds = data?.items.map((w) => w.id) ?? []
  const allSelected = pageIds.length > 0 && pageIds.every((id) => selected.has(id))

  function toggleAll() {
    setSelected((prev) => {
      const next = new Set(prev)
      if (allSelected) {
        for (const id of pageIds) next.delete(id)
      } else {
        for (const id of pageIds) next.add(id)
      }
      return next
    })
  }

  function toggleOne(id: number) {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  async function handleBatchDelete() {
    const ids = [...selected]
    if (ids.length === 0) return
    if (!window.confirm(`确定删除选中的 ${ids.length} 个单词吗？`)) return
    try {
      await words.batchDelete(bookId, ids)
      setSelected(new Set())
      await onMutated()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    }
  }

  return (
    <div className="animate-fade-in-up">
      {/* 工具条：搜索 + 排序 + 导入 */}
      <div className="flex flex-wrap items-center gap-3 mb-5">
        <div className="relative flex-1 min-w-56 max-w-sm">
          <SearchIcon className="absolute left-4 top-1/2 -translate-y-1/2 w-4 h-4 text-charcoal/35" />
          <input
            value={q}
            onChange={(e) => onQChange(e.target.value)}
            placeholder="搜索单词或释义"
            aria-label="搜索单词或释义"
            className="w-full bg-white rounded-full border border-charcoal/10 pl-10 pr-4 py-2.5 text-sm text-charcoal placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors"
          />
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <div className="bg-sand/30 rounded-full p-1 flex" role="group" aria-label="排序字段">
            {SORTS.map((s) => (
              <button
                key={s.key}
                aria-pressed={sort === s.key}
                onClick={() => {
                  setSort(s.key)
                  setPage(1)
                }}
                className={`inline-flex items-center px-4 py-1.5 rounded-full text-sm font-medium transition-all duration-200 whitespace-nowrap ${
                  sort === s.key ? 'bg-charcoal text-ivory shadow-md' : 'text-charcoal/70 hover:text-charcoal'
                }`}
              >
                {s.label}
              </button>
            ))}
          </div>
          <button
            aria-pressed={order === 'asc'}
            onClick={() => {
              setOrder((o) => (o === 'asc' ? 'desc' : 'asc'))
              setPage(1)
            }}
            className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full border border-charcoal/15 text-sm font-medium text-charcoal/70 hover:text-charcoal hover:border-charcoal/30 transition-all whitespace-nowrap"
          >
            <ArrowsUpDownIcon className="w-3.5 h-3.5" />
            {order === 'asc' ? '升序' : '降序'}
          </button>
          <button
            onClick={() => setImportOpen(true)}
            className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full border border-charcoal/15 text-sm font-medium text-charcoal/70 hover:text-charcoal hover:border-charcoal/30 transition-all whitespace-nowrap"
          >
            <UploadIcon className="w-3.5 h-3.5" />
            导入
          </button>
          <button
            onClick={onManageTags}
            className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full border border-charcoal/15 text-sm font-medium text-charcoal/70 hover:text-charcoal hover:border-charcoal/30 transition-all whitespace-nowrap"
          >
            <TagIcon className="w-3.5 h-3.5" />
            标签管理
          </button>
        </div>
      </div>

      {/* 批量操作条：选中后出现 */}
      {selected.size > 0 && (
        <div className="flex items-center gap-3 mb-4 px-5 py-3 rounded-2xl bg-sand/40 border border-sand">
          <span className="text-sm text-charcoal/70 tabular-nums">
            已选 <span className="font-semibold text-charcoal">{selected.size}</span> 项
          </span>
          <button
            onClick={handleBatchDelete}
            className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full border border-charcoal/10 text-charcoal/60 text-xs font-medium hover:border-red-300 hover:text-red-400 transition-colors whitespace-nowrap"
          >
            <TrashIcon className="w-3.5 h-3.5" />
            删除所选
          </button>
          <button
            onClick={() => setTagPickerOpen(true)}
            className="inline-flex items-center gap-1.5 px-4 py-1.5 rounded-full border border-charcoal/10 text-charcoal/60 text-xs font-medium hover:border-clay hover:text-clay transition-colors whitespace-nowrap"
          >
            <TagIcon className="w-3.5 h-3.5" />
            打标签
          </button>
        </div>
      )}

      {error && <p className="text-red-600 text-center mb-4">{error}</p>}

      <div
        key={dataVersion}
        className="bg-white rounded-2xl shadow-sm border border-charcoal/5 overflow-hidden animate-fade-in-up"
      >
        {!data ? (
          <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>
        ) : data.items.length === 0 ? (
          <p className="text-center text-charcoal/40 py-24">
            {debouncedQ ? '没有匹配的单词' : '这一页没有单词'}
          </p>
        ) : (
          <>
            {/* 桌面表格 */}
            <table className="hidden md:table w-full text-left">
              <thead>
                <tr className="text-xs font-bold text-charcoal/40 uppercase tracking-widest">
                  <th className="px-5 py-5 w-12">
                    <input
                      type="checkbox"
                      checked={allSelected}
                      onChange={toggleAll}
                      aria-label="全选本页"
                      className="accent-charcoal cursor-pointer"
                    />
                  </th>
                  <th className="px-5 py-5">单词</th>
                  <th className="px-5 py-5">音标</th>
                  <th className="px-5 py-5">释义</th>
                  <th className="px-5 py-5">标签</th>
                  <th className="px-5 py-5 w-28">添加时间</th>
                  <th className="px-5 py-5 text-right w-44">操作</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map((w, i) => (
                  <tr
                    key={w.id}
                    onClick={() => onEdit(w)}
                    className={`cursor-pointer transition-colors hover:bg-sand/20 ${
                      i < data.items.length - 1 ? 'border-b border-charcoal/5' : ''
                    }`}
                  >
                    <td className="px-5 py-3.5">
                      <input
                        type="checkbox"
                        checked={selected.has(w.id)}
                        onChange={() => toggleOne(w.id)}
                        onClick={(e) => e.stopPropagation()}
                        aria-label={`选择 ${w.spelling}`}
                        className="accent-charcoal cursor-pointer"
                      />
                    </td>
                    <td className="px-5 py-3.5 font-serif text-base text-charcoal">{w.spelling}</td>
                    <td className="px-5 py-3.5 text-sm text-charcoal/45">{w.phonetic ?? '—'}</td>
                    <td className="px-5 py-3.5 text-sm text-charcoal/70 leading-relaxed">
                      {w.definitions.map((d, di) => (
                        <span key={di}>
                          {d.pos && (
                            <span className="inline-block px-1.5 py-0.5 rounded bg-sand/60 text-charcoal/60 text-[11px] font-medium mr-1 align-middle">
                              {d.pos}
                            </span>
                          )}
                          {d.meaning}
                          {di < w.definitions.length - 1 && '；'}
                        </span>
                      ))}
                    </td>
                    <td className="px-5 py-3.5">
                      {w.tags.length > 0 && (
                        <div className="flex flex-wrap gap-1">
                          {w.tags.map((id) => (
                            <span
                              key={id}
                              className="inline-block px-1.5 py-0.5 rounded bg-sage/50 text-charcoal/70 text-[11px] font-medium"
                            >
                              {tagName.get(id) ?? id}
                            </span>
                          ))}
                        </div>
                      )}
                    </td>
                    <td className="px-5 py-3.5 text-xs text-charcoal/45 whitespace-nowrap tabular-nums">
                      {formatDate(w.created_at)}
                    </td>
                    <td className="px-5 py-3.5 whitespace-nowrap">
                      <div className="flex justify-end gap-2">
                        <button
                          onClick={(e) => {
                            e.stopPropagation()
                            onEdit(w)
                          }}
                          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium hover:bg-sand/80 transition-colors whitespace-nowrap"
                        >
                          <PencilIcon className="w-3.5 h-3.5" />
                          编辑
                        </button>
                        <button
                          onClick={(e) => {
                            e.stopPropagation()
                            onDelete(w)
                          }}
                          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full border border-charcoal/10 text-charcoal/50 text-xs font-medium hover:border-red-300 hover:text-red-400 transition-colors whitespace-nowrap"
                        >
                          <TrashIcon className="w-3.5 h-3.5" />
                          删除
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* 移动端卡片 */}
            <div className="md:hidden divide-y divide-charcoal/5">
              {data.items.map((w) => (
                <div key={w.id} onClick={() => onEdit(w)} className="px-6 py-5 cursor-pointer">
                  <div className="flex items-center gap-3">
                    <input
                      type="checkbox"
                      checked={selected.has(w.id)}
                      onChange={() => toggleOne(w.id)}
                      onClick={(e) => e.stopPropagation()}
                      aria-label={`选择 ${w.spelling}`}
                      className="accent-charcoal cursor-pointer shrink-0"
                    />
                    <div className="flex items-baseline justify-between gap-3 flex-1 min-w-0">
                      <span className="font-serif text-base text-charcoal truncate">{w.spelling}</span>
                      {w.phonetic && <span className="text-xs text-charcoal/45 shrink-0">{w.phonetic}</span>}
                    </div>
                    <span className="text-[11px] text-charcoal/40 whitespace-nowrap shrink-0 tabular-nums">
                      {formatDate(w.created_at)}
                    </span>
                  </div>
                  <p className="mt-1.5 ml-9 text-sm text-charcoal/70 leading-relaxed">{summary(w)}</p>
                  {w.tags.length > 0 && (
                    <div className="mt-2 ml-9 flex flex-wrap gap-1">
                      {w.tags.map((id) => (
                        <span
                          key={id}
                          className="inline-block px-1.5 py-0.5 rounded bg-sage/50 text-charcoal/70 text-[11px] font-medium"
                        >
                          {tagName.get(id) ?? id}
                        </span>
                      ))}
                    </div>
                  )}
                  <div className="mt-3 ml-9 flex gap-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        onEdit(w)
                      }}
                      className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium whitespace-nowrap"
                    >
                      <PencilIcon className="w-3 h-3" />
                      编辑
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        onDelete(w)
                      }}
                      className="inline-flex items-center gap-1 px-3 py-1 rounded-full border border-charcoal/10 text-charcoal/50 text-xs font-medium whitespace-nowrap"
                    >
                      <TrashIcon className="w-3 h-3" />
                      删除
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
      {data && data.total_pages > 1 && (
        <Pagination page={page} totalPages={data.total_pages} onPrev={() => setPage((p) => p - 1)} onNext={() => setPage((p) => p + 1)} />
      )}

      {importOpen && (
        <ImportModal bookId={bookId} onClose={() => setImportOpen(false)} onImported={onMutated} />
      )}

      {tagPickerOpen && (
        <TagPickerModal
          bookId={bookId}
          wordIds={[...selected]}
          tags={tags}
          onClose={() => setTagPickerOpen(false)}
          onApplied={async () => {
            setTagPickerOpen(false)
            await onMutated()
          }}
          onTagsCreated={onTagsCreated}
        />
      )}

      {importOpen && (
        <ImportModal
          bookId={bookId}
          onClose={() => setImportOpen(false)}
          onImported={async () => {
            setImportOpen(false)
            setSelected(new Set())
            await onMutated()
          }}
        />
      )}
    </div>
  )
}
