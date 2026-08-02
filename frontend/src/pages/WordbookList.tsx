import { useEffect, useState } from 'react'
import { wordbooks, type Wordbook } from '../api'
import WordbookFormModal from '../components/WordbookFormModal'
import { ArrowRightIcon, BookIcon, PencilIcon, PlusIcon, TrashIcon } from '../components/Icons'

interface Props {
  onOpen: (id: number) => void
}

const COVER_COLORS = ['bg-clay', 'bg-sage', 'bg-charcoal', 'bg-sand']

// sand 是浅色，需要深色字；其余深色底用白字
const COVER_LIGHT = new Set(['bg-sand'])

export default function WordbookList({ onOpen }: Props) {
  const [books, setBooks] = useState<Wordbook[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Wordbook | null>(null)

  async function refresh() {
    try {
      setError('')
      setBooks(await wordbooks.list())
    } catch (e) {
      setError(e instanceof Error ? e.message : '加载失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    refresh()
  }, [])

  async function handleDelete(b: Wordbook) {
    if (!window.confirm(`确定删除「${b.name}」吗？其中的单词也会一并删除。`)) return
    try {
      await wordbooks.remove(b.id)
      refresh()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    }
  }

  const totalWords = books.reduce((sum, b) => sum + b.word_count, 0)
  const avgWords = books.length > 0 ? Math.round(totalWords / books.length) : 0

  return (
    <div className="min-h-screen">
      {/* 悬浮胶囊导航 */}
      <nav className="fixed w-full top-0 z-40 py-4 px-4 md:px-8">
        <div className="max-w-7xl mx-auto">
          <div className="bg-white/70 backdrop-blur-md border border-white/40 shadow-sm rounded-full px-6 py-3 flex justify-between items-center gap-3 overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <a href="#" className="flex items-center gap-2.5 shrink-0 whitespace-nowrap">
              <div className="w-8 h-8 bg-sage rounded-full flex items-center justify-center text-white">
                <BookIcon className="w-4 h-4" />
              </div>
              <span className="font-serif text-lg text-charcoal tracking-wide">
                单词<span className="text-clay italic">本</span>
              </span>
            </a>
            <button
              onClick={() => {
                setEditing(null)
                setFormOpen(true)
              }}
              className="inline-flex items-center gap-2 bg-charcoal text-ivory px-5 py-2 rounded-full text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 whitespace-nowrap shrink-0"
            >
              <PlusIcon className="w-4 h-4" />
              新建单词书
            </button>
          </div>
        </div>
      </nav>

      {/* Hero */}
      <header className="pt-32 pb-12 lg:pt-36 lg:pb-16 px-4 md:px-8 relative overflow-hidden">
        <div className="max-w-7xl mx-auto">
          <div className="animate-fade-in-up">
            <div className="inline-flex items-center gap-2 bg-sand/30 border border-sand px-3 py-1 rounded-full text-xs font-bold tracking-wide text-charcoal/70">
              <span className="w-2 h-2 rounded-full bg-clay"></span>
              WORD BOOKS
            </div>
            <h1 className="font-serif text-4xl lg:text-5xl text-charcoal leading-[1.15] mt-5 mb-4">
              我的<span className="text-clay">单词本</span>
            </h1>
            <p className="text-base text-charcoal/70 max-w-md leading-relaxed mb-6">
              集中管理你的单词书，在纸质书模式下自测记忆效果。
            </p>
            <div className="flex flex-wrap gap-4">
              {books.length === 0 && (
                <button
                  onClick={() => {
                    setEditing(null)
                    setFormOpen(true)
                  }}
                  className="bg-charcoal text-ivory px-8 py-3.5 rounded-full font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 inline-flex items-center gap-2"
                >
                  新建单词书
                  <PlusIcon className="w-4 h-4" />
                </button>
              )}
              <a
                href="#books"
                className="border border-charcoal/20 px-8 py-3.5 rounded-full font-medium text-charcoal hover:bg-white hover:border-charcoal/40 transition-all inline-flex items-center gap-2"
              >
                浏览书架
                <ArrowRightIcon className="w-4 h-4" />
              </a>
            </div>
          </div>
        </div>
      </header>

      {error && <p className="text-red-600 text-center mb-6 px-4">{error}</p>}

      {/* 统计条 */}
      {!loading && books.length > 0 && (
        <section className="py-8 border-y border-charcoal/10 bg-white/40 animate-fade-in-up">
          <div className="max-w-5xl mx-auto px-4 md:px-8 grid grid-cols-1 md:grid-cols-3 gap-8">
            {[
              { label: '单词书', value: books.length },
              { label: '单词总数', value: totalWords },
              { label: '每本平均', value: avgWords },
            ].map((s) => (
              <div key={s.label} className="text-center">
                <p className="font-serif text-4xl text-charcoal tabular-nums">{s.value}</p>
                <p className="mt-2 text-xs font-bold text-charcoal/40 uppercase tracking-widest">
                  {s.label}
                </p>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* 书架 */}
      <main className="max-w-7xl mx-auto px-4 md:px-8 py-16 lg:py-20" id="books">
        {loading ? (
          <p className="text-center text-charcoal/40 py-24 animate-pulse">加载中…</p>
        ) : books.length === 0 ? (
          <div className="max-w-md mx-auto text-center animate-fade-in-up">
            <div className="mx-auto w-20 h-20 rounded-2xl bg-sand/40 flex items-center justify-center text-clay">
              <BookIcon className="w-9 h-9" />
            </div>
            <h2 className="font-serif text-2xl text-charcoal mt-7">还没有单词书</h2>
            <p className="mt-3 text-sm text-charcoal/60 leading-relaxed">
              创建一本单词书，开始收录你的单词。
            </p>
            <button
              onClick={() => {
                setEditing(null)
                setFormOpen(true)
              }}
              className="mt-8 bg-charcoal text-ivory px-8 py-3.5 rounded-full font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 inline-flex items-center gap-2"
            >
              <PlusIcon className="w-4 h-4" />
              新建单词书
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8">
            {books.map((b, i) => (
              <div
                key={b.id}
                onClick={() => onOpen(b.id)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' || e.key === ' ') onOpen(b.id)
                }}
                className="group cursor-pointer animate-fade-in-up"
                style={{ animationDelay: `${i * 70}ms` }}
              >
                {/* 封面：参考页同款纯色块 */}
                <div
                  className={`relative overflow-hidden rounded-2xl h-32 mb-5 shadow-sm border border-charcoal/5 ${COVER_COLORS[b.id % COVER_COLORS.length]} flex items-center justify-center transition-all duration-300 group-hover:-translate-y-1 group-hover:shadow-lg group-hover:shadow-charcoal/15`}
                >
                  <span
                    className={`font-serif text-5xl ${
                      COVER_LIGHT.has(COVER_COLORS[b.id % COVER_COLORS.length])
                        ? 'text-charcoal/60'
                        : 'text-white/90'
                    }`}
                  >
                    {b.name.charAt(0).toUpperCase()}
                  </span>
                  <span className="absolute top-3 right-3 bg-white/90 text-charcoal text-xs px-2.5 py-1 rounded-full font-medium tabular-nums transition-colors duration-300 group-hover:bg-white">
                    {b.word_count} 词
                  </span>
                </div>
                {/* 内容 */}
                <div className="px-1">
                  <h2 className="font-serif text-lg text-charcoal transition-colors duration-300 group-hover:text-clay">{b.name}</h2>
                  <p className="mt-1.5 text-xs text-charcoal/60 line-clamp-2 leading-relaxed">
                    {b.description || '暂无描述'}
                  </p>
                  <div className="mt-4 flex items-center gap-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        setEditing(b)
                        setFormOpen(true)
                      }}
                      className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full bg-sand/50 text-charcoal/80 text-xs font-medium hover:bg-sand/80 transition-colors md:opacity-0 md:group-hover:opacity-100"
                    >
                      <PencilIcon className="w-3.5 h-3.5" />
                      编辑
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        handleDelete(b)
                      }}
                      className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-full border border-charcoal/10 text-charcoal/50 text-xs font-medium hover:border-red-300 hover:text-red-400 transition-colors md:opacity-0 md:group-hover:opacity-100"
                    >
                      <TrashIcon className="w-3.5 h-3.5" />
                      删除
                    </button>
                    <span className="ml-auto text-clay opacity-0 group-hover:opacity-100 transition-opacity">
                      <ArrowRightIcon className="w-5 h-5" />
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>

      {formOpen && (
        <WordbookFormModal
          initial={editing}
          onClose={() => setFormOpen(false)}
          onSaved={async () => {
            setFormOpen(false)
            await refresh()
          }}
        />
      )}
    </div>
  )
}
