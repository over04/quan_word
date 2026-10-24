import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import DictResultList from './DictResultList'

const noop = () => {}

function makeSuggestions(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    word: `word${i}`,
    summary: `释义 ${i}`,
  }))
}

function renderList(overrides: Partial<React.ComponentProps<typeof DictResultList>> = {}) {
  return render(
    <DictResultList
      query="word"
      suggestions={makeSuggestions(9)}
      presence={{}}
      loading={false}
      error=""
      onSelect={noop}
      onRetry={noop}
      {...overrides}
    />,
  )
}

/** 结果卡是带 aria-label 的按钮（加载更多/空态按钮只有文字） */
function cardCount(container: HTMLElement) {
  return container.querySelectorAll('button[aria-label]').length
}

describe('DictResultList 分页', () => {
  afterEach(cleanup)

  it('首屏只放 6 条，加载更多每次 +6', () => {
    const { container } = renderList()
    expect(cardCount(container)).toBe(6)

    fireEvent.click(screen.getByText('加载更多（还有 3 条）'))
    expect(cardCount(container)).toBe(9)
    // 全部展开后按钮消失
    expect(screen.queryByText(/加载更多/)).toBeNull()
  })

  it('换词后回到首屏条数', () => {
    const { container, rerender } = renderList()
    fireEvent.click(screen.getByText(/加载更多/))
    expect(cardCount(container)).toBe(9)

    rerender(
      <DictResultList
        query="other"
        suggestions={makeSuggestions(9)}
        presence={{}}
        loading={false}
        error=""
        onSelect={noop}
        onRetry={noop}
      />,
    )
    expect(cardCount(container)).toBe(6)
  })

  it('少于 6 条时不渲染加载更多', () => {
    const { container } = renderList({ suggestions: makeSuggestions(3) })
    expect(cardCount(container)).toBe(3)
    expect(screen.queryByText(/加载更多/)).toBeNull()
  })

  it('点击结果卡回调单词', () => {
    const onSelect = vi.fn()
    const { container } = renderList({ onSelect })
    const first = container.querySelector('button[aria-label]') as HTMLButtonElement
    fireEvent.click(first)
    expect(onSelect).toHaveBeenCalledWith('word0')
  })
})

describe('DictResultList 空态与错误态', () => {
  afterEach(cleanup)

  it('无建议时给「在词典中查看」入口', () => {
    renderList({ query: 'zzz', suggestions: [] })
    expect(screen.getByText(/在词典中查看「zzz」/)).toBeTruthy()
  })

  it('只有回显词时等同于空态', () => {
    renderList({ query: 'zzz', suggestions: [{ word: 'zzz', summary: '' }] })
    expect(screen.getByText(/在词典中查看「zzz」/)).toBeTruthy()
  })

  it('错误态的重试按钮会回调', () => {
    const onRetry = vi.fn()
    renderList({ error: '网络错误', onRetry })
    fireEvent.click(screen.getByText('重试'))
    expect(onRetry).toHaveBeenCalledTimes(1)
  })

  it('加载中且无结果时渲染骨架（不渲染结果卡）', () => {
    const { container } = renderList({ loading: true, suggestions: [] })
    expect(cardCount(container)).toBe(0)
  })
})
