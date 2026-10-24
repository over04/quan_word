import {
  dict,
  type DictEntry,
  type DictLookupResp,
  type DictProviderResp,
  type ProviderId,
} from './api'
import type { WordDraft } from './wordDraft'

/** 会话缓存有效期：后端另有 7 天正向缓存，这里只避免同一次浏览内重复请求 */
const TTL_MS = 10 * 60 * 1000
/** 缓存条数上限：超出淘汰最旧写入的条目 */
const MAX_ENTRIES = 100

interface CachedEntry {
  at: number
  data: DictLookupResp
}

const cache = new Map<string, CachedEntry>()
/** in-flight 去重：同 key 并发复用同一 Promise（abort 只影响调用方自己的等待） */
const inflight = new Map<string, Promise<DictLookupResp>>()

/** 词典源列表：单例 Promise，整个页面生命周期只请求一次；失败后清空以便重试 */
let providersPromise: Promise<DictProviderResp[]> | null = null

/**
 * 插件重载后清空会话缓存：后端已清缓存/熔断，前端继续用旧释义会像「重载没生效」。
 * 同时丢掉源列表单例，下一次 getProviders 重新拉取。
 */
export function clearDictCache() {
  cache.clear()
  inflight.clear()
  providersPromise = null
}

/** 词典源列表（单例）。 */
export function getProviders(): Promise<DictProviderResp[]> {
  if (!providersPromise) {
    providersPromise = dict.providers().catch((e: unknown) => {
      providersPromise = null
      throw e
    })
  }
  return providersPromise
}

/** 缓存 key：词形 + 源集合（排序后拼接，源顺序不影响命中） */
function cacheKey(word: string, providers: ProviderId[]): string {
  return `${word.trim().toLowerCase()}|${[...providers].sort().join(',')}`
}

/** 读缓存：过期删除；命中时把条目移到队尾（近似 LRU） */
function readCache(key: string): DictLookupResp | null {
  const hit = cache.get(key)
  if (!hit) return null
  if (Date.now() - hit.at > TTL_MS) {
    cache.delete(key)
    return null
  }
  cache.delete(key)
  cache.set(key, hit)
  return hit.data
}

/** 写缓存：重插到队尾并淘汰超限的最旧条目 */
function writeCache(key: string, data: DictLookupResp) {
  cache.delete(key)
  cache.set(key, { at: Date.now(), data })
  while (cache.size > MAX_ENTRIES) {
    const oldest = cache.keys().next()
    if (oldest.done) break
    cache.delete(oldest.value)
  }
}

/** 同步读缓存（未命中/已过期返回 null）：供 hook 判断能否不闪 loading */
export function peekDict(word: string, providers: ProviderId[]): DictLookupResp | null {
  return readCache(cacheKey(word, providers))
}

function abortError(): Error {
  const e = new Error('请求已取消')
  e.name = 'AbortError'
  return e
}

/**
 * 让调用方 signal 只影响自己的等待：底层共享请求继续跑，结果仍写入缓存，
 * 不会因为某个调用方取消而让其它等待同一 key 的调用方失败。
 */
function withSignal<T>(p: Promise<T>, signal?: AbortSignal): Promise<T> {
  if (!signal) return p
  if (signal.aborted) return Promise.reject(abortError())
  return new Promise<T>((resolve, reject) => {
    const onAbort = () => reject(abortError())
    signal.addEventListener('abort', onAbort, { once: true })
    p.then(
      (v) => {
        signal.removeEventListener('abort', onAbort)
        resolve(v)
      },
      (e: unknown) => {
        signal.removeEventListener('abort', onAbort)
        reject(e)
      },
    )
  })
}

/**
 * 聚合查询（带会话缓存 + 单飞去重）。
 * refresh=true 绕过缓存读（仍写缓存）；signal 只取消本次等待。
 */
export function lookupDict(
  word: string,
  providers: ProviderId[],
  opts?: { refresh?: boolean; signal?: AbortSignal },
): Promise<DictLookupResp> {
  const key = cacheKey(word, providers)
  if (!opts?.refresh) {
    const hit = readCache(key)
    if (hit) return Promise.resolve(hit)
  }
  const running = inflight.get(key)
  if (running) return withSignal(running, opts?.signal)
  const request = dict
    .lookup(word.trim(), providers)
    .then((data) => {
      writeCache(key, data)
      return data
    })
    .finally(() => {
      inflight.delete(key)
    })
  inflight.set(key, request)
  return withSignal(request, opts?.signal)
}

/** 词典条目 → 「添加单词」表单草稿（义项原样保留；无义项时给一条空义项占位） */
export function entryToWordDraft(entry: DictEntry): WordDraft {
  const definitions =
    entry.definitions.length > 0
      ? entry.definitions.map((d) => ({ pos: d.pos, meaning: d.meaning }))
      : [{ pos: '', meaning: '' }]
  const first = entry.examples[0]
  const example = first
    ? first.translation
      ? `${first.text}\n${first.translation}`
      : first.text
    : ''
  return {
    spelling: entry.word,
    phonetic: entry.phonetic ?? '',
    definitions,
    example,
  }
}
