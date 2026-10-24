import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { usePresence } from './usePresence'

/// 单帧推进（组件里的 entering 用两次 requestAnimationFrame 翻转）
function flushFrame() {
  act(() => {
    vi.advanceTimersByTime(16)
  })
}

describe('usePresence', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('关闭态初始不挂载', () => {
    const { result } = renderHook(() => usePresence(false, 220))
    expect(result.current.mounted).toBe(false)
    expect(result.current.closing).toBe(false)
  })

  it('打开时先 entering，两帧后转为稳定态', () => {
    const { result, rerender } = renderHook(({ open }) => usePresence(open, 220), {
      initialProps: { open: false },
    })

    rerender({ open: true })
    expect(result.current.mounted).toBe(true)
    expect(result.current.entering).toBe(true)
    expect(result.current.closing).toBe(false)

    flushFrame()
    flushFrame()
    expect(result.current.entering).toBe(false)
    expect(result.current.mounted).toBe(true)
  })

  it('关闭时先 closing 保住节点，时长到了才卸载', () => {
    const { result, rerender } = renderHook(({ open }) => usePresence(open, 220), {
      initialProps: { open: true },
    })

    rerender({ open: false })
    expect(result.current.closing).toBe(true)
    expect(result.current.mounted).toBe(true)

    // 还差一点：仍在退场
    act(() => {
      vi.advanceTimersByTime(200)
    })
    expect(result.current.mounted).toBe(true)

    act(() => {
      vi.advanceTimersByTime(40)
    })
    expect(result.current.mounted).toBe(false)
    expect(result.current.closing).toBe(false)
  })

  it('退场途中重新打开会取消卸载', () => {
    const { result, rerender } = renderHook(({ open }) => usePresence(open, 220), {
      initialProps: { open: true },
    })

    rerender({ open: false })
    act(() => {
      vi.advanceTimersByTime(100)
    })
    rerender({ open: true })

    act(() => {
      vi.advanceTimersByTime(500)
    })
    expect(result.current.mounted).toBe(true)
    expect(result.current.closing).toBe(false)
  })
})
