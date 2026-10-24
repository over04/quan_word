import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import SettingsPage from './SettingsPage'

vi.mock('../components/DictSettings', () => ({ default: () => <div>词典源内容</div> }))
vi.mock('../components/DataSettings', () => ({ default: () => <div>数据内容</div> }))

describe('SettingsPage', () => {
  afterEach(cleanup)

  it('分区导航可切换且默认词典源', () => {
    render(<SettingsPage />)

    expect(screen.getByRole('heading', { name: '设置' })).toBeTruthy()
    expect(screen.getByText('词典源内容')).toBeTruthy()
    expect(screen.getByRole('button', { name: '词典源' }).getAttribute('aria-current')).toBe('page')

    fireEvent.click(screen.getByRole('button', { name: '数据' }))

    expect(screen.getByText('数据内容')).toBeTruthy()
    expect(screen.getByRole('button', { name: '数据' }).getAttribute('aria-current')).toBe('page')
  })
})
