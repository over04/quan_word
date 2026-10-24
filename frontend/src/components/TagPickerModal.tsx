import { useState } from 'react'
import { words, type Tag } from '../api'
import { useDiscardGuard, useUnsavedChanges } from '../hooks/useUnsavedChanges'
import { useTagDraft } from '../hooks/useTagDraft'
import DiscardConfirmModal from './DiscardConfirmModal'
import Modal from './Modal'
import TagPicker from './TagPicker'

interface Props {
  bookId: number
  wordIds: number[]
  tags: Tag[]
  onClose: () => void
  /** 批量打标签成功（父级刷新数据） */
  onApplied: () => void
  /** 新建标签成功后回调（父级刷新标签列表，仅内存同步） */
  onTagsCreated: (tag: Tag) => void
}

/** 批量给选中单词打标签的弹窗：选择/新建均为草稿，点「应用」才提交 */
export default function TagPickerModal({
  bookId,
  wordIds,
  tags: all,
  onClose,
  onApplied,
  onTagsCreated,
}: Props) {
  const draft = useTagDraft([], all, onTagsCreated)
  const [error, setError] = useState('')
  const [applying, setApplying] = useState(false)
  const guard = useDiscardGuard(draft.dirty, onClose)
  useUnsavedChanges(draft.dirty)

  function requestClose() {
    if (applying) return
    guard.requestClose()
  }

  async function handleApply() {
    if (draft.selected.size === 0 || applying) return
    setApplying(true)
    setError('')
    try {
      const ids = await draft.resolve(bookId)
      await words.batchTag(bookId, wordIds, ids)
      onApplied()
    } catch (e) {
      setError(e instanceof Error ? e.message : '操作失败')
      setApplying(false)
    }
  }

  return (
    <Modal onClose={requestClose} maxWidth="max-w-lg">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <span className="text-sm">#</span>
        </div>
        <div>
          <h2 className="font-serif text-xl text-charcoal">批量打标签</h2>
          <p className="text-xs text-charcoal/50 mt-0.5">
            已选 {wordIds.length} 个单词，为它们添加标签（不清除已有标签）
          </p>
        </div>
      </div>

      <div className="mt-6">
        <TagPicker
          tags={all}
          pending={draft.pending}
          selected={draft.selected}
          onToggle={draft.toggle}
          onCreate={draft.create}
          disabled={applying}
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
          onClick={requestClose}
          disabled={applying}
          className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
        >
          取消
        </button>
        <button
          type="button"
          onClick={handleApply}
          disabled={draft.selected.size === 0 || applying}
          className="px-6 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-50"
        >
          {applying ? '应用中…' : '应用'}
        </button>
      </div>

      {guard.confirmOpen && (
        <DiscardConfirmModal onDiscard={guard.confirmDiscard} onCancel={guard.cancelDiscard} />
      )}
    </Modal>
  )
}
