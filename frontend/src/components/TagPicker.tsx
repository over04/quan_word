import { useState } from 'react'
import { tags, type Tag } from '../api'
import { PlusIcon } from './Icons'

interface Props {
  bookId: number
  /** 该书全部标签 */
  tags: Tag[]
  /** 已选标签 id */
  selected: Set<number>
  onToggle: (id: number) => void
  /** 新建标签成功后回调（父级负责刷新标签列表并决定是否选中） */
  onCreated: (tag: Tag) => void
}

const chipClass = (active: boolean) =>
  `inline-flex items-center gap-1 px-3 py-1 rounded-full text-xs font-medium transition-colors ${
    active ? 'bg-charcoal text-ivory' : 'bg-sand/50 text-charcoal/70 hover:bg-sand/80'
  }`

/** 标签多选 + 新建（无外壳，供单词表单与批量打标签弹窗复用） */
export default function TagPicker({ bookId, tags: all, selected, onToggle, onCreated }: Props) {
  const [newName, setNewName] = useState('')
  const [error, setError] = useState('')
  const [creating, setCreating] = useState(false)

  async function handleCreate() {
    const name = newName.trim()
    if (!name || creating) return
    setCreating(true)
    setError('')
    try {
      const tag = await tags.create(bookId, { name })
      setNewName('')
      onCreated(tag)
    } catch (e) {
      setError(e instanceof Error ? e.message : '创建失败')
    } finally {
      setCreating(false)
    }
  }

  return (
    <div>
      {all.length === 0 ? (
        <p className="text-xs text-charcoal/40">还没有标签，输入名称创建</p>
      ) : (
        <div className="flex flex-wrap gap-1.5">
          {all.map((t) => (
            <button
              key={t.id}
              type="button"
              aria-pressed={selected.has(t.id)}
              onClick={() => onToggle(t.id)}
              className={chipClass(selected.has(t.id))}
            >
              {t.name}
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
          placeholder="输入新标签名，回车创建"
          aria-label="新标签名"
          className="flex-1 min-w-0 px-3.5 py-2 rounded-full bg-white border border-charcoal/15 text-sm text-charcoal placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors"
        />
        <button
          type="button"
          onClick={handleCreate}
          disabled={creating || !newName.trim()}
          className="shrink-0 inline-flex items-center gap-1 px-4 py-2 rounded-full bg-sand/60 text-charcoal/70 text-xs font-medium hover:bg-sand/90 transition-colors disabled:opacity-50"
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
