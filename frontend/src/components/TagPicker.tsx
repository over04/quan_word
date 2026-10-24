import { useState } from 'react'
import type { Tag } from '../api'
import { idTagKey, type PendingTag, type TagKey } from '../hooks/useTagDraft'
import { PlusIcon } from './Icons'

interface Props {
  /** 该书已落库标签 */
  tags: Tag[]
  /** 待创建标签（仅本地草稿） */
  pending: PendingTag[]
  /** 已选标签 key 集合（id: 或 new:） */
  selected: Set<TagKey>
  onToggle: (key: TagKey) => void
  /** 新建标签（父级负责去重与加入草稿） */
  onCreate: (name: string) => void
  disabled?: boolean
}

const chipClass = (active: boolean, isPending: boolean) =>
  `inline-flex items-center gap-1 min-h-11 px-3.5 rounded-full text-xs font-medium transition-colors ${
    active
      ? 'bg-charcoal text-ivory'
      : isPending
        ? 'border border-dashed border-clay/60 text-charcoal/60 hover:bg-sand/60'
        : 'bg-sand/50 text-charcoal/70 hover:bg-sand/80'
  }`

/** 标签多选 + 新建草稿（无外壳，供单词表单 / 快速标签 / 批量打标签复用；不发网络请求） */
export default function TagPicker({ tags: all, pending, selected, onToggle, onCreate, disabled }: Props) {
  const [newName, setNewName] = useState('')
  const [error, setError] = useState('')

  function handleCreate() {
    const name = newName.trim()
    if (!name || disabled) return
    if (name.length > 20) {
      setError('标签名不能超过 20 个字符')
      return
    }
    setError('')
    onCreate(name)
    setNewName('')
  }

  return (
    <div>
      {all.length === 0 && pending.length === 0 ? (
        <p className="text-xs text-charcoal/40">还没有标签，输入名称创建</p>
      ) : (
        <div className="flex flex-wrap gap-1.5">
          {all.map((t) => (
            <button
              key={t.id}
              type="button"
              aria-pressed={selected.has(idTagKey(t.id))}
              disabled={disabled}
              onClick={() => onToggle(idTagKey(t.id))}
              className={chipClass(selected.has(idTagKey(t.id)), false)}
            >
              {t.name}
            </button>
          ))}
          {pending.map((p) => (
            <button
              key={p.key}
              type="button"
              aria-pressed={selected.has(p.key)}
              disabled={disabled}
              onClick={() => onToggle(p.key)}
              title="保存后创建"
              className={chipClass(selected.has(p.key), true)}
            >
              {p.name}
            </button>
          ))}
        </div>
      )}
      <div className="mt-3 flex gap-2">
        <input
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              handleCreate()
            }
          }}
          disabled={disabled}
          placeholder="输入新标签名，回车加入草稿"
          aria-label="新标签名"
          className="flex-1 min-w-0 min-h-11 px-3.5 rounded-full bg-white/45 border border-charcoal/12 text-sm text-charcoal placeholder:text-charcoal/30 transition-colors hover:bg-white/65 focus:bg-white/80 disabled:opacity-50"
        />
        <button
          type="button"
          onClick={handleCreate}
          disabled={disabled || !newName.trim()}
          className="shrink-0 inline-flex items-center gap-1 min-h-11 px-4 rounded-full bg-sand/60 text-charcoal/70 text-xs font-medium hover:bg-sand/90 active:scale-95 transition-all disabled:opacity-50"
          aria-label="创建标签"
        >
          <PlusIcon className="w-3.5 h-3.5" />
          创建
        </button>
      </div>
      {error && <p className="mt-2 text-xs text-red-500">{error}</p>}
    </div>
  )
}
