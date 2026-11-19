import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { dict, type DictProviderResp } from '../api'
import { clearDictCache } from '../dictCache'
import DictDetailPage from './DictDetailPage'

vi.mock('../api', () => ({ dict: { lookup: vi.fn(), providers: vi.fn() } }))
vi.mock('../hooks/useWordPresence', () => ({ useWordPresence: () => ({}) }))
vi.mock('./AmbientBackground', () => ({ default: () => null }))

const provider: DictProviderResp = {
  id: 'example-source', label: '示例词典', short_label: '示例', description: '',
  version: '1.0.0', enabled: true, capabilities: ['lookup'],
}
const mocked = vi.mocked(dict)

function openDetail() {
  return render(<DictDetailPage word="apple" allBooks={[]} presenceVersion={0}
    onSave={vi.fn()} onCreateBook={vi.fn()} onClose={vi.fn()} />)
}

describe('词典详情', () => {
  beforeEach(() => {
    clearDictCache()
    vi.resetAllMocks()
  })
  beforeEach(() => {
    Object.defineProperty(HTMLElement.prototype, 'scrollTo', { configurable: true, value: vi.fn() })
  })
  afterEach(() => {
    cleanup()
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollTo')
  })

  it('首次无来源后重新打开时读取新增插件', async () => {
    mocked.providers.mockResolvedValueOnce([]).mockResolvedValueOnce([provider])
    mocked.lookup.mockResolvedValue({ word: 'apple', entries: [], errors: [] })
    const first = openDetail()
    await screen.findByText('没有已启用的词典来源')
    expect(mocked.lookup).not.toHaveBeenCalled()
    first.unmount()

    openDetail()
    await screen.findByText('没有查到这个词')
    expect(mocked.lookup).toHaveBeenCalledWith('apple', [provider.id], undefined)
  })

  it('展示上游失败，重试绕过缓存', async () => {
    mocked.providers.mockResolvedValue([provider])
    mocked.lookup.mockResolvedValueOnce({ word: 'apple', entries: [], errors: [
      { provider: provider.id, label: provider.label, message: '请求超时' },
    ] }).mockResolvedValueOnce({ word: 'apple', entries: [], errors: [] })
    openDetail()

    await screen.findByText('示例词典：请求超时')
    fireEvent.click(screen.getByRole('button', { name: '重试' }))
    await screen.findByText('没有查到这个词')
    expect(mocked.lookup).toHaveBeenLastCalledWith('apple', [provider.id], { refresh: true })
  })
})
