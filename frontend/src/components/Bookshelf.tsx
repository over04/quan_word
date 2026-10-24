import type { Wordbook } from '../api'
import { ArrowRightIcon, BookIcon, PencilIcon, PlusIcon, TrashIcon } from './Icons'

interface Props {
  books: Wordbook[]
  loading: boolean
  onOpen: (id: number) => void
  onCreate: () => void
  onEdit: (book: Wordbook) => void
  onDelete: (book: Wordbook) => void
}

/** 封面装帧方案：按 book.id 稳定映射（同一本书永远是同一套装帧）。
 *
 * 6 套都是「布面 + 撞色布脊」（如 clay 面 + 炭黑脊），这是精装书最直白的识别特征；
 * 配色只用调色板里的 ivory / charcoal / clay / sage / sand，字色随底色明暗切换
 * （CSS 的 --bk-ink 系列变量），无渐变、无 emoji。
 */
const COVER_SCHEMES = [
  'charcoal-clay',
  'clay-charcoal',
  'sage-charcoal',
  'sand-clay',
  'ivory-clay',
  'sage-clay',
] as const

/** 空书架的点缀字形：纯排版层，只做 transform/opacity 漂移 */
const EMPTY_GLYPHS = [
  { ch: 'A', left: '6%', top: '18%', size: '2.4rem', rot: '-9deg', dur: '21s', delay: '0s' },
  { ch: 'b', left: '88%', top: '26%', size: '1.9rem', rot: '7deg', dur: '26s', delay: '-6s' },
  { ch: 'c', left: '12%', top: '72%', size: '2rem', rot: '5deg', dur: '24s', delay: '-12s' },
  { ch: 'd', left: '84%', top: '78%', size: '2.6rem', rot: '-5deg', dur: '28s', delay: '-3s' },
]

/** 书架标题：藏书章 + 册数；与查词页的 LEXICON 星盘同一套视觉语言。 */
function ShelfHeading({ count }: { count: number }) {
  return (
    <header className="shelf-heading animate-fade-in-up">
      <div className="shelf-heading-copy">
        <span className="shelf-kicker">LIBRARY</span>
        <h1 className="shelf-title">
          单词书
          <span className="shelf-title-dot" aria-hidden="true" />
        </h1>
        <span className="shelf-title-rule" aria-hidden="true" />
      </div>
      <div className="shelf-heading-side">
        <span className="shelf-count">{String(count).padStart(2, '0')}</span>
        <span className="shelf-heading-rule" aria-hidden="true" />
        <span className="shelf-emblem" aria-hidden="true">
          <span className="shelf-emblem-ring" />
          <span className="shelf-emblem-ring shelf-emblem-ring--inner" />
          <BookIcon className="shelf-emblem-icon" />
        </span>
      </div>
    </header>
  )
}

/** 单词书分区：只负责书架网格，不掺查词逻辑。
 *
 * 结构：每张卡 = <article>（入场/悬停容器）
 *   + 打开书目的 <button>（封面 + 标题 + 描述，整块可点）
 *   + 编辑/删除按钮（同级，不嵌套在可点区域里，避免 role=button 套 button）
 */
export default function Bookshelf({ books, loading, onOpen, onCreate, onEdit, onDelete }: Props) {
  // 加载态用与真实卡片同尺寸的骨架，数据到位时高度不跳（避免"加载完成抖一下"）
  if (loading) {
    return (
      <div className="shelf-home">
        <ShelfHeading count={books.length} />
        <div className="shelf-grid flex flex-wrap justify-center gap-6" aria-busy="true">
          <span className="sr-only">加载中…</span>
          {[0, 1, 2].map((i) => (
            <div key={i} aria-hidden="true" className="w-full max-w-sm">
              {/* 封面 14rem（= 真实卡片 h-56）+ mb-5 + 标题 1.75rem + 描述 + 操作区 2.75rem */}
              <div className="h-56 mb-5 rounded-2xl bg-sand/45 animate-pulse" />
              <div className="px-1">
                <div className="h-7 w-2/5 rounded bg-sand/55 animate-pulse" />
                <div className="mt-1.5 h-4 w-3/5 rounded bg-sand/35 animate-pulse" />
              </div>
              <div className="mt-4 h-11" />
            </div>
          ))}
        </div>
      </div>
    )
  }

  if (books.length === 0) {
    return (
      <div className="shelf-home">
        <ShelfHeading count={0} />
        <div className="relative max-w-md mx-auto text-center animate-fade-in-up">
          <div className="pointer-events-none absolute inset-0 -z-10" aria-hidden="true">
            {EMPTY_GLYPHS.map((glyph) => (
              <span
                key={glyph.ch}
                className="ambient-glyph"
                style={
                  {
                    left: glyph.left,
                    top: glyph.top,
                    fontSize: glyph.size,
                    color: 'rgb(197 143 109 / 0.5)',
                    '--glyph-rot': glyph.rot,
                    '--glyph-dur': glyph.dur,
                    '--glyph-delay': glyph.delay,
                    '--glyph-op': '0.5',
                    '--glyph-dx': '12px',
                    '--glyph-dy': '-18px',
                  } as React.CSSProperties
                }
              >
                {glyph.ch}
              </span>
            ))}
          </div>
          <div className="relative mx-auto w-20 h-20 rounded-2xl bg-sand/40 flex items-center justify-center text-clay">
            <span className="shelf-empty-orbit" aria-hidden="true" />
            <span
              aria-hidden="true"
              className="shelf-pulse absolute inset-0 rounded-2xl border border-clay/30"
            />
            <BookIcon className="w-9 h-9" />
          </div>
          <h2 className="font-serif text-2xl text-charcoal mt-7">还没有单词书</h2>
          <p className="mt-3 text-sm text-charcoal/60 leading-relaxed">
            创建一本单词书，开始收录你的单词。
          </p>
          <button
            onClick={onCreate}
            className="home-nav-cta mt-8 bg-charcoal text-ivory px-8 rounded-full font-medium hover:bg-charcoal/90 shadow-lg shadow-charcoal/10 inline-flex items-center gap-2"
          >
            <PlusIcon className="w-4 h-4" />
            新建单词书
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="shelf-home">
      <ShelfHeading count={books.length} />
      <div className="shelf-grid flex flex-wrap items-start justify-center gap-6">
        {books.map((book, index) => {
          const scheme = COVER_SCHEMES[book.id % COVER_SCHEMES.length]
          return (
            <article
              key={book.id}
              style={{ animationDelay: `${index * 80}ms` }}
              className="shelf-card group animate-book-rise w-full max-w-sm"
            >
              <button
                type="button"
                onClick={() => onOpen(book.id)}
                aria-label={`打开《${book.name}》`}
                className="block w-full text-left rounded-2xl"
              >
                <div
                  className={`book-cover book-cover--${scheme} relative overflow-hidden rounded-2xl h-56 mb-5 shadow-sm border border-charcoal/5`}
                >
                  {/* 装帧层次：书脊（竖排书名）→ 书口纸页 → 压印双框 → 书签带 → 折角 */}
                  <span className="book-spine" aria-hidden="true">
                    <span className="book-spine-text">{book.name}</span>
                  </span>
                  <span className="book-pages" aria-hidden="true" />
                  <span className="book-frame" aria-hidden="true" />
                  <span className="book-ribbon" aria-hidden="true" />
                  <span className="book-fold" aria-hidden="true" />
                  {/* 书面：衬线大首字（压印/烫金）+ 横线母题 + 版权行 */}
                  <span className="book-letter font-serif" aria-hidden="true">
                    {book.name.charAt(0).toUpperCase()}
                  </span>
                  <span className="book-rule" aria-hidden="true" />
                  <span className="book-colophon">{book.word_count} 词</span>
                  <span className="book-catalog" aria-hidden="true">
                    No. {String(index + 1).padStart(2, '0')}
                  </span>
                  <span className="book-shine" aria-hidden="true" />
                </div>
                <div className="px-1">
                  <h2 className="font-serif text-lg text-charcoal transition-colors duration-300 group-hover:text-clay">
                    {book.name}
                  </h2>
                  <p className="mt-1.5 text-xs text-charcoal/65 line-clamp-2 leading-relaxed">
                    {book.description || '暂无描述'}
                  </p>
                </div>
              </button>

              <div className="book-actions px-1 mt-4 flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => onEdit(book)}
                  className="inline-flex items-center gap-1.5 min-h-11 px-4 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium hover:bg-sand/80"
                >
                  <PencilIcon className="w-3.5 h-3.5" />
                  编辑
                </button>
                <button
                  type="button"
                  onClick={() => onDelete(book)}
                  className="inline-flex items-center gap-1.5 min-h-11 px-4 rounded-full border border-charcoal/10 text-charcoal/55 text-xs font-medium hover:border-red-300 hover:text-red-400"
                >
                  <TrashIcon className="w-3.5 h-3.5" />
                  删除
                </button>
                <span
                  aria-hidden="true"
                  className="ml-auto text-clay opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity"
                >
                  <ArrowRightIcon className="w-5 h-5" />
                </span>
              </div>
            </article>
          )
        })}
        <button
          type="button"
          onClick={onCreate}
          style={{ animationDelay: `${books.length * 80}ms` }}
          className="shelf-create group w-full max-w-sm h-56 flex flex-col items-center justify-center gap-3 animate-book-rise"
        >
          <span className="shelf-create-spine" aria-hidden="true" />
          <span className="shelf-create-pages" aria-hidden="true" />
          <span className="shelf-create-frame" aria-hidden="true" />
          <span className="shelf-create-plus">
            <PlusIcon className="shelf-plus w-5 h-5" />
          </span>
          <span className="shelf-create-label">新建单词书</span>
          <span className="shelf-create-fold" aria-hidden="true" />
        </button>
      </div>
    </div>
  )
}
