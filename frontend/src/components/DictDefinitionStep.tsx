import type { DictEntry, Wordbook } from '../api'
import { CheckIcon } from './Icons'

interface Props {
  entry: DictEntry
  book: Wordbook
  selected: Set<number>
  saving: boolean
  error: string
  onToggle: (index: number) => void
  onSave: () => void
}

/** 加入词书第二步：只勾选需要的义项，默认第一条。 */
export default function DictDefinitionStep({
  entry,
  book,
  selected,
  saving,
  error,
  onToggle,
  onSave,
}: Props) {
  return (
    <>
      <div className="max-w-3xl mx-auto px-4 md:px-8 pt-6 pb-32 animate-step-in">
        <div className="flex items-baseline gap-2 flex-wrap">
          <h2 className="font-serif text-2xl text-charcoal break-words">{entry.word}</h2>
          <span className="text-xs text-charcoal/55">加入《{book.name}》</span>
        </div>
        {entry.phonetic && <p className="mt-1 text-sm text-charcoal/60">{entry.phonetic}</p>}
        <p className="mt-3 text-xs text-charcoal/50">已选 {selected.size} 条，点击义项可取消</p>

        <div className="mt-4 space-y-2">
          {entry.definitions.map((definition, index) => {
            const on = selected.has(index)
            return (
              <button
                key={index}
                type="button"
                onClick={() => onToggle(index)}
                aria-pressed={on}
                className={`w-full min-h-12 text-left rounded-xl border px-3.5 py-3 flex items-start gap-3 transition-all active:scale-[0.99] ${
                  on
                    ? 'border-charcoal/25 bg-white'
                    : 'border-charcoal/5 bg-white/50 hover:border-charcoal/15 active:bg-sand/30'
                }`}
              >
                <span
                  className={`mt-0.5 w-5 h-5 rounded-md border flex items-center justify-center shrink-0 transition-colors ${
                    on ? 'bg-charcoal border-charcoal text-ivory' : 'border-charcoal/20 text-transparent'
                  }`}
                >
                  <CheckIcon className="w-3 h-3" />
                </span>
                <span className="min-w-0 text-sm text-charcoal/80 leading-relaxed">
                  {definition.pos && (
                    <span className="mr-1.5 text-[11px] font-medium text-charcoal/50">
                      {definition.pos}
                    </span>
                  )}
                  {definition.meaning}
                </span>
              </button>
            )
          })}
        </div>
      </div>

      <div className="fixed bottom-0 inset-x-0 z-20 border-t border-charcoal/10 bg-ivory">
        <div className="max-w-3xl mx-auto px-4 md:px-8 py-3 pb-[calc(0.75rem+env(safe-area-inset-bottom))]">
          {error && <p className="mb-2 text-xs text-red-600 text-center" role="alert">{error}</p>}
          <button
            type="button"
            onClick={onSave}
            disabled={saving || selected.size === 0}
            aria-busy={saving}
            className="w-full inline-flex items-center justify-center gap-2 min-h-12 px-6 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 active:scale-[0.99] transition-all disabled:opacity-40 disabled:active:scale-100 disabled:cursor-not-allowed"
          >
            {saving ? '保存中…' : `加入 ${selected.size} 条释义`}
          </button>
        </div>
      </div>
    </>
  )
}
