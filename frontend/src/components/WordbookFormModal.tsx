import { useState } from 'react'
import { wordbooks, type Wordbook } from '../api'
import Modal from './Modal'
import { BookIcon, PencilIcon } from './Icons'

interface Props {
  initial: Wordbook | null
  onClose: () => void
  onSaved: () => void
}

const inputClass =
  'w-full px-4 py-2.5 rounded-xl bg-white border border-charcoal/15 text-charcoal text-sm placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors'

export default function WordbookFormModal({ initial, onClose, onSaved }: Props) {
  const [name, setName] = useState(initial?.name ?? '')
  const [description, setDescription] = useState(initial?.description ?? '')
  const [error, setError] = useState('')
  const [saving, setSaving] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (!name.trim()) {
      setError('书名不能为空')
      return
    }
    setSaving(true)
    setError('')
    try {
      const payload = {
        name: name.trim(),
        description: description.trim() || undefined,
      }
      if (initial) {
        await wordbooks.update(initial.id, payload)
      } else {
        await wordbooks.create(payload)
      }
      onSaved()
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
      setSaving(false)
    }
  }

  return (
    <Modal onClose={onClose} maxWidth="max-w-md">
      <form onSubmit={handleSubmit}>
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
            {initial ? <PencilIcon className="w-4 h-4" /> : <BookIcon className="w-4 h-4" />}
          </div>
          <h2 className="font-serif text-xl text-charcoal">
            {initial ? '编辑单词书' : '新建单词书'}
          </h2>
        </div>

        <div className="mt-7 space-y-5">
          <div>
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">名称 *</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="如：CET-4 高频词"
              autoFocus
              className={inputClass}
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">描述</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
              placeholder="这本书是关于什么的"
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
    </Modal>
  )
}
