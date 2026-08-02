import { useState } from 'react'
import { words, type Definition, type Word } from '../api'
import { PencilIcon, PlusIcon } from './Icons'

interface Props {
  bookId: number
  initial: Word | null
  onClose: () => void
  onSaved: () => void
}

const inputClass =
  'w-full px-4 py-2.5 rounded-xl bg-white border border-charcoal/15 text-charcoal text-sm placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors'

/** 词性枚举：下拉选择，防止手输错误 */
const POS_OPTIONS = [
  'n.',
  'v.',
  'adj.',
  'adv.',
  'prep.',
  'conj.',
  'pron.',
  'num.',
  'art.',
  'interj.',
  'aux.',
  'abbr.',
  'phr.',
]

export default function WordFormModal({ bookId, initial, onClose, onSaved }: Props) {
  const [spelling, setSpelling] = useState(initial?.spelling ?? '')
  const [phonetic, setPhonetic] = useState(initial?.phonetic ?? '')
  const [definitions, setDefinitions] = useState<Definition[]>(
    initial?.definitions ?? [{ pos: '', meaning: '' }],
  )
  const [example, setExample] = useState(initial?.example ?? '')
  const [error, setError] = useState('')
  const [saving, setSaving] = useState(false)

  function updateDef(i: number, patch: Partial<Definition>) {
    setDefinitions((ds) => ds.map((d, idx) => (idx === i ? { ...d, ...patch } : d)))
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!spelling.trim()) {
      setError('单词不能为空')
      return
    }
    if (definitions.length === 0 || definitions.some((d) => !d.meaning.trim())) {
      setError('至少需要一个释义，且释义内容不能为空')
      return
    }
    setSaving(true)
    setError('')
    try {
      const payload = {
        spelling: spelling.trim(),
        phonetic: phonetic.trim() || undefined,
        definitions: definitions.map((d) => ({ pos: d.pos.trim(), meaning: d.meaning.trim() })),
        example: example.trim() || undefined,
      }
      if (initial) {
        await words.update(initial.id, payload)
      } else {
        await words.create(bookId, payload)
      }
      onSaved()
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
      setSaving(false)
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-charcoal/25 backdrop-blur-sm animate-fade-in"
      onClick={onClose}
    >
      <form
        onSubmit={handleSubmit}
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-lg max-h-[90vh] overflow-y-auto bg-white rounded-[2rem] border border-charcoal/5 shadow-2xl shadow-charcoal/15 p-8 md:p-9 animate-pop-in"
      >
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
            {initial ? <PencilIcon className="w-4 h-4" /> : <PlusIcon className="w-4 h-4" />}
          </div>
          <h2 className="font-serif text-xl text-charcoal">{initial ? '编辑单词' : '添加单词'}</h2>
        </div>

        <div className="mt-7 space-y-5">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <div className="md:col-span-2">
              <label className="block text-xs font-medium text-charcoal/60 mb-1.5">单词 *</label>
              <input
                value={spelling}
                onChange={(e) => setSpelling(e.target.value)}
                placeholder="abandon"
                autoFocus
                className={inputClass}
              />
            </div>
            <div>
              <label className="block text-xs font-medium text-charcoal/60 mb-1.5">音标</label>
              <input
                value={phonetic}
                onChange={(e) => setPhonetic(e.target.value)}
                placeholder="/əˈbændən/"
                className={inputClass}
              />
            </div>
          </div>

          <div>
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">释义 *</label>
            <div className="space-y-2.5">
              {definitions.map((d, i) => (
                <div key={i} className="flex gap-2">
                  <select
                    value={d.pos}
                    onChange={(e) => updateDef(i, { pos: e.target.value })}
                    className={`w-32 shrink-0 px-3 py-2.5 rounded-xl bg-white border border-charcoal/15 text-sm focus:border-clay focus:outline-none transition-colors ${
                      d.pos === '' ? 'text-charcoal/35' : 'text-charcoal'
                    }`}
                    aria-label="词性"
                  >
                    <option value="" disabled hidden>
                      词性
                    </option>
                    {POS_OPTIONS.map((p) => (
                      <option key={p} value={p}>
                        {p}
                      </option>
                    ))}
                  </select>
                  <input
                    value={d.meaning}
                    onChange={(e) => updateDef(i, { meaning: e.target.value })}
                    placeholder="释义内容"
                    className={`${inputClass} min-w-0`}
                  />
                  {definitions.length > 1 && (
                    <button
                      type="button"
                      onClick={() => setDefinitions((ds) => ds.filter((_, idx) => idx !== i))}
                      className="shrink-0 w-10 rounded-xl text-charcoal/35 hover:text-red-400 hover:bg-red-50 transition-colors"
                      title="删除此行"
                    >
                      ✕
                    </button>
                  )}
                </div>
              ))}
            </div>
            <button
              type="button"
              onClick={() => setDefinitions((ds) => [...ds, { pos: '', meaning: '' }])}
              className="mt-3 text-xs font-medium text-clay hover:text-charcoal transition-colors"
            >
              + 添加释义
            </button>
          </div>

          <div>
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">例句</label>
            <textarea
              value={example}
              onChange={(e) => setExample(e.target.value)}
              rows={2}
              placeholder="Don't abandon hope."
              className={inputClass}
            />
          </div>
        </div>

        {error && (
          <p className="mt-5 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500">
            {error}
          </p>
        )}

        <div className="mt-8 flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all"
          >
            取消
          </button>
          <button
            type="submit"
            disabled={saving}
            className="px-6 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-50"
          >
            {saving ? '保存中…' : '保存'}
          </button>
        </div>
      </form>
    </div>
  )
}
