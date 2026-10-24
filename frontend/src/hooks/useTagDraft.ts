import { useCallback, useMemo, useState } from 'react'
import { tags as tagApi, type Tag } from '../api'

/** 标签草稿 key：已落库标签用 id:，待创建标签用 new:name */
export type TagKey = `id:${number}` | `new:${string}`

/** 待创建标签（保存时才真正创建） */
export interface PendingTag {
  key: `new:${string}`
  name: string
}

export function idTagKey(id: number): `id:${number}` {
  return `id:${id}`
}

export function newTagKey(name: string): `new:${string}` {
  return `new:${name}`
}

/**
 * 标签选择草稿：点击只改本地，保存时 resolve() 才创建待创建标签并返回全量 id。
 *
 * - 新建名称与已有标签（大小写不敏感）同名时直接选中已有项，不重复创建；
 * - resolve() 逐个创建 pending 标签；中途失败时已创建的标签会保留并转为普通标签，
 *   调用方可直接重试（不会重复创建）。
 */
export function useTagDraft(
  initialIds: number[],
  allTags: Tag[],
  onCreated?: (tag: Tag) => void,
) {
  const [selected, setSelected] = useState<Set<TagKey>>(() => new Set(initialIds.map(idTagKey)))
  const [pending, setPending] = useState<PendingTag[]>([])

  const toggle = useCallback((key: TagKey) => {
    setSelected((prev) => {
      const next = new Set(prev)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }, [])

  /** 输入新标签：同名已有/待创建标签直接选中，否则加入待创建草稿并选中 */
  const create = useCallback(
    (raw: string) => {
      const name = raw.trim()
      if (!name) return
      const lower = name.toLowerCase()
      const existing = allTags.find((t) => t.name.toLowerCase() === lower)
      if (existing) {
        setSelected((prev) => new Set(prev).add(idTagKey(existing.id)))
        return
      }
      const pendingMatch = pending.find((p) => p.name.toLowerCase() === lower)
      if (pendingMatch) {
        setSelected((prev) => new Set(prev).add(pendingMatch.key))
        return
      }
      const key = newTagKey(name)
      setPending((prev) => (prev.some((p) => p.key === key) ? prev : [...prev, { key, name }]))
      setSelected((prev) => new Set(prev).add(key))
    },
    [allTags, pending],
  )

  /** 保存前调用：创建 pending 标签并返回全量标签 id（顺序与 selected 一致） */
  const resolve = useCallback(
    async (bookId: number): Promise<number[]> => {
      const snapshot = [...selected]
      const mapping = new Map<TagKey, number>()
      try {
        for (const p of pending) {
          const tag = await tagApi.create(bookId, { name: p.name })
          mapping.set(p.key, tag.id)
          onCreated?.(tag)
        }
      } finally {
        if (mapping.size > 0) {
          setPending((prev) => prev.filter((p) => !mapping.has(p.key)))
          setSelected((prev) => {
            const next = new Set<TagKey>()
            for (const k of prev) {
              const id = mapping.get(k)
              next.add(id !== undefined ? idTagKey(id) : k)
            }
            return next
          })
        }
      }
      return snapshot
        .map((k) => (k.startsWith('id:') ? Number(k.slice(3)) : mapping.get(k)))
        .filter((id): id is number => id !== undefined && Number.isFinite(id))
    },
    [pending, selected, onCreated],
  )

  const dirty = useMemo(() => {
    if (pending.length > 0) return true
    if (selected.size !== initialIds.length) return true
    for (const id of initialIds) {
      if (!selected.has(idTagKey(id))) return true
    }
    return false
  }, [pending, selected, initialIds])

  return { selected, pending, toggle, create, resolve, dirty }
}
