import { useState } from 'react'
import { tags, type Tag } from '../api'
import Modal from './Modal'
import { CheckIcon, PencilIcon, PlusIcon, TrashIcon } from './Icons'

interface Props {
  bookId: number
  tags: Tag[]
  onClose: () => void
  /** 任何变更（新建/重命名/删除）成功后回调（父级刷新标签与列表） */
  onChanged: () => void
}

const inputClass =
  'w-full px-3.5 py-2 rounded-xl bg-white border border-charcoal/15 text-charcoal text-sm placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors'

/** 管理当前单词书的全部标签：新建 / 行内重命名 / 删除 */
export default function TagManageModal({ bookId, tags: all, onClose, onChanged }: Props) {
  const [newName, setNewName] = useState('')
  const [creating, setCreating] = useState(false)
  const [error, setError] = useState('')
  // 行内重命名：标签 id → 编辑中状态
  const [renaming, setRenaming] = useState<number | null>(null)
  const [draft, setDraft] = useState('')
  const [busyId, setBusyId] = useState<number | null>(null)

  async function handleCreate() {
    const name = newName.trim()
    if (!name || creating) return
    setCreating(true)
    setError('')
    try {
      await tags.create(bookId, { name })
      setNewName('')
      onChanged()
    } catch (e) {
      setError(e instanceof Error ? e.message : '创建失败')
    } finally {
      setCreating(false)
    }
  }

  function startRename(t: Tag) {
    setRenaming(t.id)
    setDraft(t.name)
    setError('')
  }

  async function handleRename(id: number) {
    const name = draft.trim()
    if (!name || busyId !== null) return
    setBusyId(id)
    setError('')
    try {
      await tags.update(bookId, id, { name })
      setRenaming(null)
      onChanged()
    } catch (e) {
      setError(e instanceof Error ? e.message : '重命名失败')
    } finally {
      setBusyId(null)
    }
  }

  async function handleDelete(t: Tag) {
    if (!window.confirm(`确定删除标签「${t.name}」吗？将同时从 ${t.word_count} 个单词上移除。`)) return
    setBusyId(t.id)
    setError('')
    try {
      await tags.remove(bookId, t.id)
      onChanged()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    } finally {
      setBusyId(null)
    }
  }

  return (
    <Modal onClose={onClose} maxWidth="max-w-md">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <span className="text-sm">#</span>
        </div>
        <h2 className="font-serif text-xl text-charcoal">管理标签</h2>
      </div>

      <div className="mt-6 flex gap-2">
        <input
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              handleCreate()
            }
          }}
          placeholder="新标签名，回车创建"
          aria-label="新标签名"
          className={inputClass}
        />
        <button
          onClick={handleCreate}
          disabled={creating || !newName.trim()}
          className="shrink-0 inline-flex items-center gap-1 px-4 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all disabled:opacity-50"
        >
          <PlusIcon className="w-3.5 h-3.5" />
          新建
        </button>
      </div>

      {error && (
        <p className="mt-3 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500">
          {error}
        </p>
      )}

      <div className="mt-5">
        {all.length === 0 ? (
          <p className="text-center text-charcoal/40 py-10 text-sm">还没有标签</p>
        ) : (
          <ul className="divide-y divide-charcoal/5 max-h-80 overflow-y-auto rounded-2xl border border-charcoal/5">
            {all.map((t) => (
              <li key={t.id} className="px-4 py-3 flex items-center gap-3">
                {renaming === t.id ? (
                  <>
                    <input
                      value={draft}
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault()
                          handleRename(t.id)
                        }
                        if (e.key === 'Escape') setRenaming(null)
                      }}
                      autoFocus
                      aria-label={`重命名 ${t.name}`}
                      className={`${inputClass} flex-1`}
                    />
                    <button
                      onClick={() => handleRename(t.id)}
                      disabled={busyId !== null}
                      className="shrink-0 w-8 h-8 rounded-full text-charcoal/60 hover:bg-sand/50 hover:text-charcoal transition-colors disabled:opacity-50"
                      aria-label="保存重命名"
                    >
                      <CheckIcon className="w-4 h-4 mx-auto" />
                    </button>
                    <button
                      onClick={() => setRenaming(null)}
                      className="shrink-0 w-8 h-8 rounded-full text-charcoal/40 hover:bg-sand/50 hover:text-charcoal transition-colors"
                      aria-label="取消重命名"
                    >
                      ✕
                    </button>
                  </>
                ) : (
                  <>
                    <span className="flex-1 min-w-0 text-sm text-charcoal truncate">{t.name}</span>
                    <span className="shrink-0 text-xs text-charcoal/40 tabular-nums">{t.word_count} 词</span>
                    <button
                      onClick={() => startRename(t)}
                      className="shrink-0 w-8 h-8 rounded-full text-charcoal/40 hover:bg-sand/50 hover:text-charcoal transition-colors"
                      aria-label={`重命名 ${t.name}`}
                    >
                      <PencilIcon className="w-3.5 h-3.5 mx-auto" />
                    </button>
                    <button
                      onClick={() => handleDelete(t)}
                      disabled={busyId !== null}
                      className="shrink-0 w-8 h-8 rounded-full text-charcoal/40 hover:bg-red-50 hover:text-red-400 transition-colors disabled:opacity-50"
                      aria-label={`删除 ${t.name}`}
                    >
                      <TrashIcon className="w-3.5 h-3.5 mx-auto" />
                    </button>
                  </>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="mt-8 flex justify-end">
        <button
          onClick={onClose}
          className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all"
        >
          关闭
        </button>
      </div>
    </Modal>
  )
}
