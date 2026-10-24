import { useEffect, useRef, useState } from 'react'

/**
 * 「关掉要有退场、打开要有入场」的挂载开关。
 *
 * 条件渲染（{open && <X/>}）在关闭时是**瞬间卸载**：节点直接从 DOM 里消失，
 * 任何 transition / animation 都来不及跑；打开时同理 —— 元素以终态插入，
 * CSS transition 不会从「收起态」开始插值。
 *
 *   const p = usePresence(open, 220)
 *   {p.mounted && <X data-state={p.entering ? 'entering' : p.closing ? 'closing' : 'open'} />}
 *
 * - entering：只在「从关到开」的第一帧为 true（两帧后翻掉），
 *   样式侧用 [data-state='entering'] 写收起态，翻成 open 时 transition 才有起点；
 * - closing：关闭后继续挂载 durationMs，样式侧播退场，播完才卸载。
 */
export function usePresence(open: boolean, durationMs = 220) {
  const [mounted, setMounted] = useState(open)
  const [closing, setClosing] = useState(false)
  const [entering, setEntering] = useState(false)
  /** 上一次的 open，用来区分「首次挂载」与「真的从开到关 / 从关到开」 */
  const prevRef = useRef(open)

  useEffect(() => {
    const was = prevRef.current
    prevRef.current = open

    if (open) {
      setMounted(true)
      setClosing(false)
      if (was) return
      // 刚挂载：先以收起态画一帧，再翻成展开态，transition 才跑得起来
      setEntering(true)
      let second = 0
      const first = requestAnimationFrame(() => {
        second = requestAnimationFrame(() => setEntering(false))
      })
      return () => {
        cancelAnimationFrame(first)
        if (second) cancelAnimationFrame(second)
      }
    }

    // 首次渲染就是关闭态：没有「曾经打开过」的节点要收
    if (!was) return

    setEntering(false)
    setClosing(true)
    const timer = window.setTimeout(() => {
      setMounted(false)
      setClosing(false)
    }, durationMs)
    return () => window.clearTimeout(timer)
  }, [open, durationMs])

  return { mounted, closing, entering }
}
