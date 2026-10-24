import { renderHook, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { dict } from '../api'
import { clearSuggestCache, useDictSuggest } from './useDictSuggest'

vi.mock('../api', () => ({
  dict: { suggest: vi.fn() },
}))

const suggest = vi.mocked(dict.suggest)

describe('useDictSuggest', () => {
  beforeEach(() => {
    clearSuggestCache()
    suggest.mockReset()
  })

  it('空结果不写缓存：导入插件后同一个词能重新搜出来', async () => {
    suggest.mockResolvedValueOnce({ word: 'game', suggestions: [] })
    const first = renderHook(() => useDictSuggest('game'))

    await waitFor(() => expect(suggest).toHaveBeenCalledTimes(1), { timeout: 2000 })
    await waitFor(() => expect(first.result.current.loading).toBe(false), { timeout: 2000 })
    expect(first.result.current.suggestions).toEqual([{ word: 'game', summary: '' }])
    first.unmount()

    suggest.mockResolvedValueOnce({
      word: 'game',
      suggestions: [{ word: 'game', summary: 'n. 游戏' }],
    })
    const second = renderHook(() => useDictSuggest('game'))

    await waitFor(() => expect(suggest).toHaveBeenCalledTimes(2), { timeout: 2000 })
    await waitFor(
      () => expect(second.result.current.suggestions[0]?.summary).toBe('n. 游戏'),
      { timeout: 2000 },
    )
  })
})
