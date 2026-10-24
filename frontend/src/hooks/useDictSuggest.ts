import { useCallback, useEffect, useState } from 'react'
import { dict, type DictSuggestion } from '../api'

/** 输入防抖：避免每个字符都请求建议接口。 */
const DEBOUNCE_MS = 400
/** 少于两个字符不查。 */
const MIN_LENGTH = 2
/** 建议结果缓存 10 分钟（后端也有缓存，这里省掉往返）。 */
const CACHE_TTL_MS = 10 * 60 * 1000

interface CacheEntry {
  at: number
  items: DictSuggestion[]
}

const cache = new Map<string, CacheEntry>()

/** 模糊查词建议：输入变化自动查询，返回列表与加载态。 */
export function useDictSuggest(input: string) {
  const [suggestions, setSuggestions] = useState<DictSuggestion[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [version, setVersion] = useState(0)
  const word = input.trim()

  useEffect(() => {
    if (word.length < MIN_LENGTH) {
      setSuggestions([])
      setLoading(false)
      setError('')
      return
    }
    const key = word.toLowerCase()
    const hit = cache.get(key)
    if (hit && Date.now() - hit.at < CACHE_TTL_MS) {
      setSuggestions(hit.items)
      setLoading(false)
      setError('')
      return
    }

    const controller = new AbortController()
    setLoading(true)
    setError('')
    const timer = window.setTimeout(() => {
      dict
        .suggest(word, { signal: controller.signal })
        .then((res) => {
          const items: DictSuggestion[] =
            res.suggestions.length > 0
              ? res.suggestions
              : [{ word, summary: '' }]
          cache.set(key, { at: Date.now(), items })
          setSuggestions(items)
          setLoading(false)
        })
        .catch((e: unknown) => {
          if (!controller.signal.aborted) {
            setSuggestions([])
            setLoading(false)
            setError(e instanceof Error ? e.message : '查词失败')
          }
        })
    }, DEBOUNCE_MS)

    return () => {
      controller.abort()
      window.clearTimeout(timer)
    }
  }, [word, version])

  const refresh = useCallback(() => {
    cache.delete(word.toLowerCase())
    setVersion((value) => value + 1)
  }, [word])

  return { suggestions, loading, error, refresh }
}
