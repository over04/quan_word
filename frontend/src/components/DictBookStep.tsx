import type { Wordbook } from '../api'
import { BookIcon, CheckIcon, PlusIcon } from './Icons'

interface Props {
  books: Wordbook[]
  presentBookIds: Set<number>
  onChoose: (book: Wordbook) => void
  onCreate: () => void
}

/** 加入词书的第一步：先选书；已收录的书不可重复选。 */
export default function DictBookStep({ books, presentBookIds, onChoose, onCreate }: Props) {
  if (books.length === 0) {
    return (
      <div className="max-w-3xl mx-auto px-4 md:px-8 pt-6 pb-16 animate-step-in">
        <div className="py-20 text-center">
          <div className="mx-auto w-16 h-16 rounded-2xl bg-sand/50 flex items-center justify-center text-clay">
            <BookIcon className="w-7 h-7" />
          </div>
          <p className="mt-5 text-sm text-charcoal/60">还没有单词书</p>
          <button
            type="button"
            onClick={onCreate}
            className="mt-5 inline-flex items-center gap-2 min-h-12 px-6 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 active:scale-95 transition-all"
          >
            <PlusIcon className="w-4 h-4" />
            新建单词书
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="max-w-3xl mx-auto px-4 md:px-8 pt-6 pb-16 animate-step-in">
      <p className="mb-4 text-sm text-charcoal/55">选择要加入的单词书</p>
      <div className="space-y-3">
        {books.map((book) => {
          const present = presentBookIds.has(book.id)
          return (
            <button
              key={book.id}
              type="button"
              onClick={() => onChoose(book)}
              disabled={present}
              className="w-full min-h-16 text-left bg-white rounded-2xl border border-charcoal/5 shadow-sm px-4 py-3.5 flex items-center gap-3 hover:border-charcoal/15 hover:shadow-md active:scale-[0.99] active:bg-sand/20 transition-all disabled:opacity-60 disabled:cursor-default disabled:active:scale-100"
            >
              <span className="w-10 h-10 rounded-xl bg-sage/50 text-charcoal/60 flex items-center justify-center shrink-0">
                <BookIcon className="w-5 h-5" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="flex items-center gap-2">
                  <span className="font-serif text-lg text-charcoal truncate">{book.name}</span>
                  {present && (
                    <span className="shrink-0 inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-sage/60 text-[11px] text-charcoal/65">
                      <CheckIcon className="w-3 h-3" />
                      已收录
                    </span>
                  )}
                </span>
                <span className="block mt-0.5 text-xs text-charcoal/55 truncate">
                  {book.word_count} 词{book.description ? ` · ${book.description}` : ''}
                </span>
              </span>
              {/* 已收录的行不可点：不给 › 箭头，避免暗示还能进入 */}
              {!present && <span className="shrink-0 text-charcoal/25">›</span>}
            </button>
          )
        })}
      </div>
      <button
        type="button"
        onClick={onCreate}
        className="mt-3 w-full min-h-14 rounded-2xl border border-dashed border-charcoal/20 text-sm font-medium text-charcoal/60 hover:border-charcoal/40 hover:text-charcoal active:bg-sand/30 active:scale-[0.99] transition-all inline-flex items-center justify-center gap-2"
      >
        <PlusIcon className="w-4 h-4" />
        新建单词书
      </button>
    </div>
  )
}
