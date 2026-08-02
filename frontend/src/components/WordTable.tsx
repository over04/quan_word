import { useEffect, useState } from 'react'
import { words, type Page, type Word } from '../api'
import Pagination from './Pagination'
import { ArrowsUpDownIcon, PencilIcon, SearchIcon, TrashIcon } from './Icons'

interface Props {
  bookId: number
  /** 增删改后递增，触发重新加载 */
  refreshKey: number
  onEdit: (w: Word) => void
  onDelete: (w: Word) => void
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

/** 列表模式：书内搜索 + 排序 + 分页（自包含数据加载） */
export default function WordTable({ bookId, refreshKey, onEdit, onDelete }: Props) {
  const [q, setQ] = useState('')
  const [debouncedQ, setDebouncedQ] = useState('')
  const [sort, setSort] = useState('created_at')
  const [order, setOrder] = useState<'asc' | 'desc'>('asc')
  const [page, setPage] = useState(1)
  const [data, setData] = useState<Page<Word> | null>(null)
  const [error, setError] = useState('')

  // 搜索防抖：停止输入 400ms 后生效
  useEffect(() => {
    const t = window.setTimeout(() => setDebouncedQ(q.trim()), 400)
    return () => window.clearTimeout(t)
  }, [q])

  // 查询加载：搜索词 / 排序 / 页码 / 数据变更时重新请求
  useEffect(() => {
    let alive = true
    words
      .query(bookId, page, PAGE_SIZE, {
        q: debouncedQ || undefined,
        sort,
        order,
      })
      .then((r) => {
        if (alive) setData(r)
      })
      .catch((e) => {
        if (alive) setError(e instanceof Error ? e.message : '加载失败')
      })
    return () => {
      alive = false
    }
  }, [bookId, debouncedQ, sort, order, page, refreshKey])

  // 删除后当前页可能空：回退上一页
  useEffect(() => {
    if (data && data.items.length === 0 && page > 1) setPage((p) => p - 1)
  }, [data, page])

  function onQChange(v: string) {
    setQ(v)
    setPage(1)
  }

  return (
    <div className="animate-fade-in-up">
      {/* 工具条：搜索 + 排序 */}
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
        </div>
      </div>

      {error && <p className="text-red-600 text-center mb-4">{error}</p>}

      <div className="bg-white rounded-2xl shadow-sm border border-charcoal/5 overflow-hidden">
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
                  <th className="px-7 py-5">单词</th>
                  <th className="px-7 py-5">音标</th>
                  <th className="px-7 py-5">释义</th>
                  <th className="px-7 py-5 text-right w-44">操作</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map((w, i) => (
                  <tr
                    key={w.id}
                    className={`transition-colors hover:bg-sand/20 ${
                      i < data.items.length - 1 ? 'border-b border-charcoal/5' : ''
                    }`}
                  >
                    <td className="px-7 py-3.5 font-serif text-base text-charcoal">{w.spelling}</td>
                    <td className="px-7 py-3.5 text-sm text-charcoal/45">{w.phonetic ?? '—'}</td>
                    <td className="px-7 py-3.5 text-sm text-charcoal/70 leading-relaxed">
                      {summary(w)}
                    </td>
                    <td className="px-7 py-3.5 whitespace-nowrap">
                      <div className="flex justify-end gap-2">
                        <button
                          onClick={() => onEdit(w)}
                          className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium hover:bg-sand/80 transition-colors whitespace-nowrap"
                        >
                          <PencilIcon className="w-3.5 h-3.5" />
                          编辑
                        </button>
                        <button
                          onClick={() => onDelete(w)}
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
                <div key={w.id} className="px-6 py-5">
                  <div className="flex items-baseline justify-between gap-3">
                    <span className="font-serif text-base text-charcoal">{w.spelling}</span>
                    {w.phonetic && <span className="text-xs text-charcoal/45">{w.phonetic}</span>}
                  </div>
                  <p className="mt-1 text-sm text-charcoal/70 leading-relaxed">{summary(w)}</p>
                  <div className="mt-3 flex gap-2">
                    <button
                      onClick={() => onEdit(w)}
                      className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium whitespace-nowrap"
                    >
                      <PencilIcon className="w-3 h-3" />
                      编辑
                    </button>
                    <button
                      onClick={() => onDelete(w)}
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
    </div>
  )
}
