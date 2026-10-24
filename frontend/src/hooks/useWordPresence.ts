import { useEffect, useMemo, useState } from 'react'
import { wordbooks, type WordPresenceBook } from '../api'

interface PresenceCacheEntry {
  at: number
  books: WordPresenceBook[]
}

const CACHE_TTL_MS = 5 * 60 * 1000
const cache = new Map<string, PresenceCacheEntry>()

/** 清空存在性缓存（新增单词后调用，避免列表显示旧状态）。 */
export function clearPresenceCache() {
  cache.clear()
}

/** 批量查询单词是否已在各词书中，返回小写拼写 → 词书列表；version 变化时强制重查。 */
export function useWordPresence(words: string[], version = 0) {
  const wordsKey = useMemo(() => {
    const unique = new Set<string>()
    for (const word of words) {
      const value = word.trim()
      if (value) unique.add(value)
    }
    return [...unique].join('\u0000')
  }, [words])
  const [presence, setPresence] = useState<Record<string, WordPresenceBook[]>>({})

  useEffect(() => {
    const unique = wordsKey ? wordsKey.split('\u0000') : []
    if (unique.length === 0) {
      setPresence({})
      return
    }

    let alive = true
    const controller = new AbortController()
    const missing = unique.filter((word) => {
      const hit = cache.get(word.toLowerCase())
      return !hit || Date.now() - hit.at >= CACHE_TTL_MS
    })

    const load = async () => {
      await Promise.all(
        missing.map(async (word) => {
          try {
            const res = await wordbooks.presence(word, { signal: controller.signal })
            cache.set(word.toLowerCase(), { at: Date.now(), books: res.books })
          } catch {
            // 失败不写缓存：写空会让「已在词书里」的单词 5 分钟内显示成未收录
            // （用户看到的「已收录」状态必须可靠，宁可下次重试）
          }
        }),
      )
      if (!alive) return
      const next: Record<string, WordPresenceBook[]> = {}
      for (const word of unique) {
        next[word.toLowerCase()] = cache.get(word.toLowerCase())?.books ?? []
      }
      setPresence(next)
    }

    void load()
    return () => {
      alive = false
      controller.abort()
    }
  }, [wordsKey, version])

  return presence
}
