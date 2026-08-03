import { useState } from 'react'
import { words, type Definition, type Tag, type Word } from '../api'
import Modal from './Modal'
import TagPicker from './TagPicker'
import { PencilIcon, PlusIcon } from './Icons'

interface Props {
  bookId: number
  initial: Word | null
  onClose: () => void
  onSaved: () => void
  /** 该书全部标签 */
  tags: Tag[]
  /** 新建标签成功后回调（父级刷新标签列表） */
  onTagsCreated: (tag: Tag) => void
}

const inputClass =
  'w-full px-4 py-2.5 rounded-xl bg-white border border-charcoal/15 text-charcoal text-sm placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors'

/** 词性枚举：下拉选择，防止手输错误；C 可数 / U 不可数（名词），vt./vi. 及物/不及物（动词） */
const POS_OPTIONS = [
  'n.',
  'C',
  'U',
  'CU',
  'v.',
  'vt.',
  'vi.',
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

export default function WordFormModal({ bookId, initial, onClose, onSaved, tags, onTagsCreated }: Props) {
  const [spelling, setSpelling] = useState(initial?.spelling ?? '')
  const [phonetic, setPhonetic] = useState(initial?.phonetic ?? '')
  const [definitions, setDefinitions] = useState<Definition[]>(
    initial?.definitions ?? [{ pos: '', meaning: '' }],
  )
  const [example, setExample] = useState(initial?.example ?? '')
  const [selectedTagIds, setSelectedTagIds] = useState<Set<number>>(new Set(initial?.tags ?? []))
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
        tags: [...selectedTagIds],
      }
      if (initial) {
        await words.update(bookId, initial.id, payload)
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
    <Modal onClose={onClose} maxWidth="max-w-lg">
      <form onSubmit={handleSubmit}>
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
                    {d.pos && !POS_OPTIONS.includes(d.pos) && (
                      <option value={d.pos}>{d.pos}</option>
                    )}
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

          <div>
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">标签</label>
            <TagPicker
              bookId={bookId}
              tags={tags}
              selected={selectedTagIds}
              onToggle={(id) =>
                setSelectedTagIds((prev) => {
                  const n = new Set(prev)
                  if (n.has(id)) n.delete(id)
                  else n.add(id)
                  return n
                })
              }
              onCreated={(tag) => {
                onTagsCreated(tag)
                setSelectedTagIds((prev) => new Set(prev).add(tag.id))
              }}
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
    </Modal>
  )
}
