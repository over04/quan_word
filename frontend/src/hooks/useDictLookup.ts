import { useCallback, useEffect, useRef, useState } from 'react'
import type { DictEntry, DictProviderErrorResp, ProviderId } from '../api'
import { lookupDict, peekDict } from '../dictCache'

/** 输入停止多久后发起查询（毫秒） */
const DEBOUNCE_MS = 500
/** 少于该长度不查询（单字母对词典源没有意义） */
const MIN_LENGTH = 2

interface DictLookupState {
  entries: DictEntry[]
  errors: DictProviderErrorResp[]
  loading: boolean
  error: string
  /** 当前结果是否已对应当前查询（首帧/加载中/失败为 false，避免闪「未找到」） */
  settled: boolean
}

const EMPTY: DictLookupState = { entries: [], errors: [], loading: false, error: '', settled: false }

/**
 * 词典聚合查询：500ms 防抖 + 最小长度 + 会话缓存直读（命中不闪 loading）。
 * 参数变化/卸载时 abort 上一次等待；refresh() 跳过缓存并立即重查。
 * onResult 在每次成功拿到结果（含缓存命中、entries 为空）后调用一次；
 * 请求失败/参数不满足时不调用。
 */
export function useDictLookup(
  word: string,
  providers: ProviderId[],
  enabled: boolean,
  onResult?: (entries: DictEntry[]) => void,
  /** 防抖毫秒：输入框场景用默认 500；打开即查（详情页）传 0，不让骨架白等半秒 */
  debounceMs: number = DEBOUNCE_MS,
) {
  const [state, setState] = useState<DictLookupState>(EMPTY)
  // 回调走 ref：父级每次渲染的新函数不会让查询 effect 重跑
  const onResultRef = useRef(onResult)
  onResultRef.current = onResult
  // 刷新计数：自增触发重新查询
  const [refreshTick, setRefreshTick] = useState(0)
  // 本次 effect 是否强制回源（refresh() 设置，effect 读取后清除）
  const forceRef = useRef(false)
  // providers 数组每次渲染可能是新引用：依赖用 join 后的稳定字符串，取值走 ref
  const providersRef = useRef(providers)
  providersRef.current = providers
  const providersKey = providers.join(',')
  const trimmed = word.trim()

  useEffect(() => {
    const ids = providersRef.current
    if (!enabled || trimmed.length < MIN_LENGTH || ids.length === 0) {
      setState(EMPTY)
      return
    }
    const force = forceRef.current
    forceRef.current = false
    if (!force) {
      // 已缓存：直接出结果，不闪 loading
      const hit = peekDict(trimmed, ids)
      if (hit) {
        setState({ entries: hit.entries, errors: hit.errors, loading: false, error: '', settled: true })
        onResultRef.current?.(hit.entries)
        return
      }
    }
    const ctrl = new AbortController()
    let alive = true
    setState((s) => ({ ...s, loading: true, error: '', settled: false }))
    const timer = window.setTimeout(
      () => {
        lookupDict(trimmed, ids, { refresh: force, signal: ctrl.signal })
          .then((r) => {
            if (!alive) return
            setState({
              entries: r.entries,
              errors: r.errors,
              loading: false,
              error: '',
              settled: true,
            })
            onResultRef.current?.(r.entries)
          })
          .catch((e: unknown) => {
            if (!alive) return
            if (e instanceof Error && e.name === 'AbortError') return
            setState({
              entries: [],
              errors: [],
              loading: false,
              error: e instanceof Error ? e.message : '查询失败',
              settled: false,
            })
          })
      },
      force ? 0 : debounceMs,
    )
    return () => {
      alive = false
      window.clearTimeout(timer)
      ctrl.abort()
    }
  }, [trimmed, providersKey, enabled, refreshTick, debounceMs])

  /** 强制刷新：跳过后端与本地缓存，立即重查当前词 */
  const refresh = useCallback(() => {
    forceRef.current = true
    setRefreshTick((n) => n + 1)
  }, [])

  return {
    entries: state.entries,
    errors: state.errors,
    loading: state.loading,
    error: state.error,
    settled: state.settled,
    refresh,
  }
}
