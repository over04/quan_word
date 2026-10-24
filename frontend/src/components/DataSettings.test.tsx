import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { clearDictCache } from '../dictCache'
import { clearSearchHistory } from '../searchHistory'
import { toast } from '../toast'
import DataSettings from './DataSettings'

vi.mock('../dictCache', () => ({ clearDictCache: vi.fn() }))
vi.mock('../searchHistory', () => ({ clearSearchHistory: vi.fn() }))
vi.mock('../toast', () => ({ toast: vi.fn() }))

describe('DataSettings', () => {
  beforeEach(() => {
    vi.resetAllMocks()
  })

  afterEach(cleanup)

  it('清理查词缓存与搜索历史', () => {
    render(<DataSettings />)

    fireEvent.click(screen.getByRole('button', { name: '清除查词缓存' }))
    fireEvent.click(screen.getByRole('button', { name: '清除搜索历史' }))

    expect(clearDictCache).toHaveBeenCalledTimes(1)
    expect(clearSearchHistory).toHaveBeenCalledTimes(1)
    expect(toast).toHaveBeenCalledWith('已清除查词缓存')
    expect(toast).toHaveBeenCalledWith('已清除搜索历史')
  })
})
