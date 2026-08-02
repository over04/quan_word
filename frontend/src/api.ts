export interface Definition {
  pos: string
  meaning: string
}

export interface Wordbook {
  id: number
  name: string
  description: string
  icon: string
  word_count: number
}

export interface Word {
  id: number
  wordbook_id: number
  spelling: string
  phonetic: string | null
  definitions: Definition[]
  example: string | null
}

export interface Page<T> {
  items: T[]
  total: number
  page: number
  page_size: number
  total_pages: number
}

export interface CreateWordbookReq {
  name: string
  description?: string
  icon?: string
}

export interface CreateWordReq {
  spelling: string
  phonetic?: string
  definitions: Definition[]
  example?: string
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: init?.body ? { 'Content-Type': 'application/json' } : undefined,
    ...init,
  })
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
  update: (id: number, req: CreateWordReq) =>
    api<Word>(`/api/words/${id}`, { method: 'PUT', body: JSON.stringify(req) }),
  remove: (id: number) => api<void>(`/api/words/${id}`, { method: 'DELETE' }),
}
