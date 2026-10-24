import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import DictResultList from './DictResultList'

const noop = () => {}

let intersectionCallback: IntersectionObserverCallback | null = null

/** 结果列表用 IntersectionObserver 自动续页；测试里把回调抓出来手动触发。 */
class MockIntersectionObserver {
  constructor(callback: IntersectionObserverCallback) {
    intersectionCallback = callback
  }
  observe() {}
  disconnect() {}
  unobserve() {}
  takeRecords() {
    return []
  }
}

function scrollToBottom() {
  act(() => {
    intersectionCallback?.([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)
  })
}

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

/** 结果卡是带 aria-label 的按钮（空态/错误态按钮只有文字） */
function cardCount(container: HTMLElement) {
  return container.querySelectorAll('button[aria-label]').length
}

beforeEach(() => {
  intersectionCallback = null
  vi.stubGlobal('IntersectionObserver', MockIntersectionObserver)
})

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

describe('DictResultList 分页', () => {
  it('首屏 6 条，滚到底自动续 6 条直到全部出现', () => {
    const { container } = renderList({ suggestions: makeSuggestions(24) })
    expect(cardCount(container)).toBe(6)

    scrollToBottom()
    expect(cardCount(container)).toBe(12)

    scrollToBottom()
    expect(cardCount(container)).toBe(18)

    scrollToBottom()
    expect(cardCount(container)).toBe(24)
    expect(container.querySelector('.dict-result-sentinel')).toBeNull()
  })

  it('换词后回到首屏条数', () => {
    const { container, rerender } = renderList()
    scrollToBottom()
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

  it('少于 6 条时不渲染哨兵', () => {
    const { container } = renderList({ suggestions: makeSuggestions(3) })
    expect(cardCount(container)).toBe(3)
    expect(container.querySelector('.dict-result-sentinel')).toBeNull()
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
