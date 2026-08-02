import { useState } from 'react'
import { words, type Tag, type Word } from '../api'
import Modal from './Modal'
import TagPicker from './TagPicker'

interface Props {
  bookId: number
  word: Word
  tags: Tag[]
  onClose: () => void
  /** 标签集变更成功（父级刷新数据） */
  onChanged: () => void
  /** 新建标签成功后回调（父级刷新标签列表） */
  onTagsCreated: (tag: Tag) => void
}

/** 单词快速标签编辑弹窗：每次切换即时保存（全量替换该词标签集） */
export default function TagQuickModal({
  bookId,
  word,
  tags: all,
  onClose,
  onChanged,
  onTagsCreated,
}: Props) {
  const [selected, setSelected] = useState<Set<number>>(new Set(word.tags))
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  async function apply(next: Set<number>) {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      await words.updateTags(bookId, word.id, [...next])
      setSelected(next)
      onChanged()
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
    } finally {
      setBusy(false)
    }
  }

  function toggle(id: number) {
    const next = new Set(selected)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    apply(next)
  }

  return (
    <Modal onClose={onClose} maxWidth="max-w-sm">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <span className="font-serif text-sm">标</span>
        </div>
        <div className="min-w-0">
          <h2 className="font-serif text-xl text-charcoal truncate">{word.spelling}</h2>
          <p className="text-xs text-charcoal/50 mt-0.5">点击标签即时保存</p>
        </div>
      </div>

      <div className="mt-6">
        <TagPicker
          bookId={bookId}
          tags={all}
          selected={selected}
          onToggle={toggle}
          onCreated={(tag) => {
            onTagsCreated(tag)
            apply(new Set(selected).add(tag.id))
          }}
        />
      </div>

      {error && (
        <p className="mt-4 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500">
          {error}
        </p>
      )}

      <div className="mt-8 flex justify-end">
        <button
          onClick={onClose}
          className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all"
        >
          完成
        </button>
      </div>
    </Modal>
  )
}
