import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { getApiKey, setApiKey, verifyApiKey, wordbooks, words } from './api'

function deferredResponse() {
  let resolve!: (response: Response) => void
  const promise = new Promise<Response>((done) => { resolve = done })
  return { promise, resolve }
}

describe('访问密钥', () => {
  beforeEach(() => localStorage.clear())
  afterEach(() => vi.unstubAllGlobals())

  it.each([
    ['JSON 请求', () => wordbooks.list()],
    ['下载请求', () => words.downloadTemplate(1, 'csv')],
  ] as const)('%s的旧 401 不会删除新的密钥', async (_name, load) => {
    const old = deferredResponse()
    const fetchMock = vi.fn().mockReturnValueOnce(old.promise).mockResolvedValueOnce(Response.json([]))
    vi.stubGlobal('fetch', fetchMock)
    const inflight = load().catch((e: Error) => e.message)

    await verifyApiKey('good-key')
    expect(getApiKey()).toBeNull()
    expect(new Headers(fetchMock.mock.calls[1][1].headers).get('Authorization')).toBe('Bearer good-key')
    setApiKey('good-key')
    old.resolve(new Response(null, { status: 401 }))
    expect(await inflight).toBe('需要访问密钥')
    expect(getApiKey()).toBe('good-key')
  })

  it('只在当前密钥被拒绝时清理会话并发出登录事件', async () => {
    setApiKey('old-key')
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 401 })))
    const authRequired = vi.fn()
    window.addEventListener('qw:auth-required', authRequired, { once: true })
    try {
      await expect(verifyApiKey('incorrect')).rejects.toThrow('密钥无效')
      expect(getApiKey()).toBe('old-key')
      expect(authRequired).not.toHaveBeenCalled()
      await expect(wordbooks.list()).rejects.toThrow('需要访问密钥')
      expect(getApiKey()).toBeNull()
      expect(authRequired).toHaveBeenCalledOnce()
    } finally {
      window.removeEventListener('qw:auth-required', authRequired)
    }
  })

  it('区分格式、网络和服务响应错误', async () => {
    const fetchMock = vi.fn().mockRejectedValueOnce(new TypeError('offline'))
      .mockResolvedValueOnce(new Response('<html></html>'))
    vi.stubGlobal('fetch', fetchMock)
    await expect(verifyApiKey('中文')).rejects.toThrow('可见 ASCII')
    expect(fetchMock).not.toHaveBeenCalled()
    await expect(verifyApiKey('good-key')).rejects.toThrow('网络连接失败')
    await expect(verifyApiKey('good-key')).rejects.toThrow('服务器响应格式异常')
  })
})
