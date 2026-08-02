// 共享契约类型由后端 ts-rs 生成（frontend/src/generated/，cargo test 时刷新），此处仅转发。
import type { BatchDeleteWordsReq } from './generated/BatchDeleteWordsReq'
import type { BatchDeleteWordsResp } from './generated/BatchDeleteWordsResp'
import type { CreateWordReq } from './generated/CreateWordReq'
import type { CreateWordbookReq } from './generated/CreateWordbookReq'
import type { Definition } from './generated/Definition'
import type { ImportResp } from './generated/ImportResp'
import type { Page } from './generated/Page'
import type { Word } from './generated/Word'
import type { Wordbook } from './generated/Wordbook'

export type {
  BatchDeleteWordsReq,
  BatchDeleteWordsResp,
  CreateWordReq,
  CreateWordbookReq,
  Definition,
  ImportResp,
  Page,
  Word,
  Wordbook,
}

const KEY_STORAGE = 'qw_api_key'

/** 读取本地保存的访问密钥（无则 null）。 */
export function getApiKey(): string | null {
  return localStorage.getItem(KEY_STORAGE)
}

/** 保存/清除访问密钥（localStorage）。 */
export function setApiKey(k: string | null) {
  if (k) localStorage.setItem(KEY_STORAGE, k)
  else localStorage.removeItem(KEY_STORAGE)
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = { ...(init?.headers as Record<string, string> | undefined) }
  const key = getApiKey()
  if (key) headers['Authorization'] = `Bearer ${key}`
  // 仅 JSON 字符串 body 需要手动设 Content-Type；FormData 由浏览器自动带 boundary
  if (init?.body && typeof init.body === 'string') headers['Content-Type'] = 'application/json'
  const res = await fetch(path, { ...init, headers })
  if (res.status === 401) {
    setApiKey(null)
    window.dispatchEvent(new Event('qw:auth-required'))
    throw new Error('需要访问密钥')
  }
  if (!res.ok) {
    let msg = `请求失败 (${res.status})`
    try {
      const data = await res.json()
      if (data?.error) msg = data.error
    } catch {
      // 非 JSON 错误响应，保留默认消息
    }
    throw new Error(msg)
  }
  if (res.status === 204) return undefined as T
  return res.json() as Promise<T>
}

export const wordbooks = {
  list: () => api<Wordbook[]>('/api/wordbooks'),
  get: (id: number) => api<Wordbook>(`/api/wordbooks/${id}`),
  create: (req: CreateWordbookReq) =>
    api<Wordbook>('/api/wordbooks', { method: 'POST', body: JSON.stringify(req) }),
  update: (id: number, req: Partial<CreateWordbookReq>) =>
    api<Wordbook>(`/api/wordbooks/${id}`, { method: 'PUT', body: JSON.stringify(req) }),
  remove: (id: number) => api<void>(`/api/wordbooks/${id}`, { method: 'DELETE' }),
}

export const words = {
  /** 纸质书浏览：可选排序（id_asc/id_desc/spelling/random）；random 需 seed（确定性打乱） */
  list: (bookId: number, page: number, pageSize: number, opts?: { order?: string; seed?: string }) => {
    const order = opts?.order ? `&order=${encodeURIComponent(opts.order)}` : ''
    const seed = opts?.seed ? `&seed=${encodeURIComponent(opts.seed)}` : ''
    return api<Page<Word>>(
      `/api/wordbooks/${bookId}/words?page=${page}&page_size=${pageSize}${order}${seed}`,
    )
  },
  /** 列表模式查询：书内搜索（spelling/释义）+ 排序 + 分页 */
  query: (bookId: number, page: number, pageSize: number, opts?: { q?: string; sort?: string; order?: string }) => {
    const q = opts?.q ? `&q=${encodeURIComponent(opts.q)}` : ''
    const sort = opts?.sort ? `&sort=${encodeURIComponent(opts.sort)}` : ''
    const order = opts?.order ? `&order=${encodeURIComponent(opts.order)}` : ''
    return api<Page<Word>>(
      `/api/wordbooks/${bookId}/words/query?page=${page}&page_size=${pageSize}${q}${sort}${order}`,
    )
  },
  create: (bookId: number, req: CreateWordReq) =>
    api<Word>(`/api/wordbooks/${bookId}/words`, { method: 'POST', body: JSON.stringify(req) }),
  update: (bookId: number, id: number, req: CreateWordReq) =>
    api<Word>(`/api/wordbooks/${bookId}/words/${id}`, {
      method: 'PUT',
      body: JSON.stringify(req),
    }),
  remove: (bookId: number, id: number) =>
    api<void>(`/api/wordbooks/${bookId}/words/${id}`, { method: 'DELETE' }),
  /** 批量删除（限定归属该书） */
  batchDelete: (bookId: number, ids: number[]) =>
    api<BatchDeleteWordsResp>(`/api/wordbooks/${bookId}/words/batch-delete`, {
      method: 'POST',
      body: JSON.stringify({ ids } satisfies BatchDeleteWordsReq),
    }),
  /** 上传模板文件导入（csv / xlsx / xls / ods） */
  importFile: (bookId: number, file: File) => {
    const fd = new FormData()
    fd.append('file', file)
    return api<ImportResp>(`/api/wordbooks/${bookId}/words/import`, { method: 'POST', body: fd })
  },
  /** 下载导入模板并触发浏览器保存 */
  downloadTemplate: async (bookId: number, format: 'csv' | 'xlsx') => {
    const headers: Record<string, string> = {}
    const key = getApiKey()
    if (key) headers['Authorization'] = `Bearer ${key}`
    const res = await fetch(`/api/wordbooks/${bookId}/words/template?format=${format}`, { headers })
    if (res.status === 401) {
      setApiKey(null)
      window.dispatchEvent(new Event('qw:auth-required'))
      throw new Error('需要访问密钥')
    }
    if (!res.ok) {
      let msg = `请求失败 (${res.status})`
      try {
        const data = await res.json()
        if (data?.error) msg = data.error
      } catch {
        // 非 JSON 错误响应，保留默认消息
      }
      throw new Error(msg)
    }
    const blob = await res.blob()
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `单词导入模板.${format}`
    a.click()
    URL.revokeObjectURL(url)
  },
}
