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
let cacheGeneration = 0

/** 插件导入/删除/启停后调用：来源变了，旧的空结果/旧建议不能继续复用。 */
export function clearSuggestCache() {
  cacheGeneration += 1
  cache.clear()
}

interface SuggestResult {
  word: string
  attempt: number
  suggestions: DictSuggestion[]
  error: string
}

/** 只暴露当前输入对应的结果，等待新请求时不会提交上一个词。 */
export function useDictSuggest(input: string) {
  const [result, setResult] = useState<SuggestResult | null>(null)
  const [attempt, setAttempt] = useState(0)
  const word = input.trim()
  const key = word.toLowerCase()
  const searchable = word.length >= MIN_LENGTH
  const hit = searchable ? cache.get(key) : undefined
  const cached = hit && Date.now() - hit.at < CACHE_TTL_MS ? hit.items : undefined
  const currentResult = searchable && result?.word === word && result.attempt === attempt
    ? result
    : null

  useEffect(() => {
    if (!searchable || cached) return
    const controller = new AbortController()
    const requestGeneration = cacheGeneration
    const timer = window.setTimeout(() => {
      dict.suggest(word, { signal: controller.signal }).then((res) => {
        if (controller.signal.aborted) return
        const hasResults = res.suggestions.length > 0
        const suggestions = hasResults ? res.suggestions : [{ word, summary: '' }]
        if (hasResults && requestGeneration === cacheGeneration) {
          cache.set(key, { at: Date.now(), items: suggestions })
        }
        setResult({ word, attempt, suggestions, error: '' })
      }).catch((e: unknown) => {
        if (controller.signal.aborted) return
        setResult({ word, attempt, suggestions: [], error: e instanceof Error ? e.message : '查词失败' })
      })
    }, DEBOUNCE_MS)
    return () => {
      controller.abort()
      window.clearTimeout(timer)
    }
  }, [searchable, key, word, attempt, cached])

  const refresh = useCallback(() => {
    cache.delete(key)
    setAttempt((value) => value + 1)
  }, [key])

  return {
    suggestions: cached ?? (currentResult ? currentResult.suggestions : []),
    loading: searchable && !cached && !currentResult,
    error: currentResult && !cached ? currentResult.error : '',
    refresh,
  }
}
