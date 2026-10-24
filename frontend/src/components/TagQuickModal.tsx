import { useState } from 'react'
import { words, type Tag, type Word } from '../api'
import { useDiscardGuard, useUnsavedChanges } from '../hooks/useUnsavedChanges'
import { useTagDraft } from '../hooks/useTagDraft'
import DiscardConfirmModal from './DiscardConfirmModal'
import Modal from './Modal'
import TagPicker from './TagPicker'

interface Props {
  bookId: number
  word: Word
  tags: Tag[]
  onClose: () => void
  /** 标签集变更成功（父级刷新数据） */
  onChanged: () => void
  /** 新建标签成功后回调（父级刷新标签列表，仅内存同步） */
  onTagsCreated: (tag: Tag) => void
}

/** 单词快速标签编辑弹窗：选择只改本地草稿，点击保存才提交（全量替换该词标签集） */
export default function TagQuickModal({
  bookId,
  word,
  tags: all,
  onClose,
  onChanged,
  onTagsCreated,
}: Props) {
  const draft = useTagDraft(word.tags, all, onTagsCreated)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const guard = useDiscardGuard(draft.dirty, onClose)
  useUnsavedChanges(draft.dirty)

  function requestClose() {
    if (busy) return
    guard.requestClose()
  }

  async function handleSave() {
    if (busy || !draft.dirty) return
    setBusy(true)
    setError('')
    try {
      const ids = await draft.resolve(bookId)
      await words.updateTags(bookId, word.id, ids)
      onChanged()
      onClose()
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
      setBusy(false)
    }
  }

  return (
    <Modal onClose={requestClose} maxWidth="max-w-sm">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <span className="font-serif text-sm">标</span>
        </div>
        <div className="min-w-0">
          <h2 className="font-serif text-xl text-charcoal truncate">{word.spelling}</h2>
        </div>
      </div>

      <div className="mt-6">
        <TagPicker
          tags={all}
          pending={draft.pending}
          selected={draft.selected}
          onToggle={draft.toggle}
          onCreate={draft.create}
          disabled={busy}
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
          disabled={busy}
          className="min-h-11 px-5 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 active:scale-95 transition-all disabled:opacity-50"
        >
          取消
        </button>
        <button
          type="button"
          onClick={handleSave}
          disabled={busy || !draft.dirty}
          className="min-h-11 px-6 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 active:scale-95 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-50"
        >
          {busy ? '保存中…' : '保存'}
        </button>
      </div>

      {guard.confirmOpen && (
        <DiscardConfirmModal onDiscard={guard.confirmDiscard} onCancel={guard.cancelDiscard} />
      )}
    </Modal>
  )
}
