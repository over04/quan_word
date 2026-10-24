import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import HomeNav from './HomeNav'

afterEach(cleanup)

describe('HomeNav', () => {
  it('renders 查词 / 单词书 / 设置 and reports tab changes', () => {
    const onTab = vi.fn()
    render(<HomeNav tab="dict" onTab={onTab} onCreate={() => {}} />)

    expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toEqual([
      '查词',
      '单词书',
      '设置',
    ])
    fireEvent.click(screen.getByRole('tab', { name: '设置' }))
    expect(onTab).toHaveBeenCalledWith('settings')
  })

  it('hides 新建单词书 on the settings tab', () => {
    render(<HomeNav tab="settings" onTab={() => {}} onCreate={() => {}} />)
    expect(screen.queryByRole('button', { name: /新建单词书/ })).toBeNull()
  })
})
