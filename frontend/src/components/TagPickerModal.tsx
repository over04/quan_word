import { useState } from 'react'
import { words, type Tag } from '../api'
import Modal from './Modal'
import TagPicker from './TagPicker'

interface Props {
  bookId: number
  wordIds: number[]
  tags: Tag[]
  onClose: () => void
  /** 批量打标签成功（父级刷新数据） */
  onApplied: () => void
  /** 新建标签成功后回调（父级刷新标签列表） */
  onTagsCreated: (tag: Tag) => void
}

/** 批量给选中单词打标签的弹窗 */
export default function TagPickerModal({
  bookId,
  wordIds,
  tags: all,
  onClose,
  onApplied,
  onTagsCreated,
}: Props) {
  const [selected, setSelected] = useState<Set<number>>(new Set())
  const [error, setError] = useState('')
  const [applying, setApplying] = useState(false)

  async function handleApply() {
    if (selected.size === 0 || applying) return
    setApplying(true)
    setError('')
    try {
      await words.batchTag(bookId, wordIds, [...selected])
      onApplied()
    } catch (e) {
      setError(e instanceof Error ? e.message : '操作失败')
      setApplying(false)
    }
  }

  return (
    <Modal onClose={onClose} maxWidth="max-w-lg">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <span className="text-sm">#</span>
        </div>
        <div>
          <h2 className="font-serif text-xl text-charcoal">批量打标签</h2>
          <p className="text-xs text-charcoal/50 mt-0.5">已选 {wordIds.length} 个单词，为它们添加标签（不清除已有标签）</p>
        </div>
      </div>

      <div className="mt-6">
        <TagPicker
          bookId={bookId}
          tags={all}
          selected={selected}
          onToggle={(id) =>
            setSelected((prev) => {
              const n = new Set(prev)
              if (n.has(id)) n.delete(id)
              else n.add(id)
              return n
            })
          }
          onCreated={(tag) => {
            onTagsCreated(tag)
            setSelected((prev) => new Set(prev).add(tag.id))
          }}
        />
      </div>

      {error && (
        <p className="mt-4 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500">
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
          type="button"
          onClick={handleApply}
          disabled={selected.size === 0 || applying}
          className="px-6 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-50"
        >
          {applying ? '应用中…' : '应用'}
        </button>
      </div>
    </Modal>
  )
}
