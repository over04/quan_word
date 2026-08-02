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
  create: (req: CreateWordbookReq) =>
    api<Wordbook>('/api/wordbooks', { method: 'POST', body: JSON.stringify(req) }),
  update: (id: number, req: Partial<CreateWordbookReq>) =>
    api<Wordbook>(`/api/wordbooks/${id}`, { method: 'PUT', body: JSON.stringify(req) }),
  remove: (id: number) => api<void>(`/api/wordbooks/${id}`, { method: 'DELETE' }),
}

export const words = {
  list: (bookId: number, page: number, pageSize: number) =>
    api<Page<Word>>(`/api/wordbooks/${bookId}/words?page=${page}&page_size=${pageSize}`),
  create: (bookId: number, req: CreateWordReq) =>
    api<Word>(`/api/wordbooks/${bookId}/words`, { method: 'POST', body: JSON.stringify(req) }),
  update: (id: number, req: CreateWordReq) =>
    api<Word>(`/api/words/${id}`, { method: 'PUT', body: JSON.stringify(req) }),
  remove: (id: number) => api<void>(`/api/words/${id}`, { method: 'DELETE' }),
}
