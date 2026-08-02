import type { Page, Word } from '../api'
import Pagination from './Pagination'
import { PencilIcon, TrashIcon } from './Icons'

interface Props {
  data: Page<Word> | null
  page: number
  onPrev: () => void
  onNext: () => void
  onEdit: (w: Word) => void
  onDelete: (w: Word) => void
}

function summary(w: Word): string {
  return w.definitions.map((d) => (d.pos ? `${d.pos} ${d.meaning}` : d.meaning)).join('；')
}

export default function WordTable({ data, page, onPrev, onNext, onEdit, onDelete }: Props) {
  if (!data) return <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>
  if (data.items.length === 0) {
    return <p className="text-center text-charcoal/40 py-24">这一页没有单词</p>
  }
  return (
    <div key={page} className="animate-fade-in">
      <div className="bg-white rounded-2xl shadow-sm border border-charcoal/5 overflow-hidden">
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
                <td className="px-7 py-3.5 text-sm text-charcoal/70 line-clamp-2 max-w-md leading-relaxed">
                  {summary(w)}
                </td>
                <td className="px-7 py-3.5">
                  <div className="flex justify-end gap-2">
                    <button
                      onClick={() => onEdit(w)}
                      className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium hover:bg-sand/80 transition-colors"
                    >
                      <PencilIcon className="w-3.5 h-3.5" />
                      编辑
                    </button>
                    <button
                      onClick={() => onDelete(w)}
                      className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full border border-charcoal/10 text-charcoal/50 text-xs font-medium hover:border-red-300 hover:text-red-400 transition-colors"
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
              <p className="mt-1 text-sm text-charcoal/70 line-clamp-2 leading-relaxed">
                {summary(w)}
              </p>
              <div className="mt-3 flex gap-2">
                <button
                  onClick={() => onEdit(w)}
                  className="inline-flex items-center gap-1 px-3 py-1 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium"
                >
                  <PencilIcon className="w-3 h-3" />
                  编辑
                </button>
                <button
                  onClick={() => onDelete(w)}
                  className="inline-flex items-center gap-1 px-3 py-1 rounded-full border border-charcoal/10 text-charcoal/50 text-xs font-medium"
                >
                  <TrashIcon className="w-3 h-3" />
                  删除
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
      <Pagination page={page} totalPages={data.total_pages} onPrev={onPrev} onNext={onNext} />
    </div>
  )
}
