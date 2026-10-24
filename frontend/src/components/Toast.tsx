import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { subscribeToast } from '../toast'

/**
 * 轻提示宿主：每个页面壳层挂一个（同一时刻只有一个页面）。
 * role=status + aria-live=polite 让读屏也能播报。
 */
/** 停留 2.4s 后开始退场，退场 0.2s 再卸载（不留「啪」一下消失） */
const HOLD_MS = 2400
const EXIT_MS = 200

export default function ToastHost() {
  const [message, setMessage] = useState('')
  const [seq, setSeq] = useState(0)
  const [closing, setClosing] = useState(false)

  useEffect(
    () =>
      subscribeToast((msg) => {
        setMessage(msg)
        setSeq((n) => n + 1)
      }),
    [],
  )

  useEffect(() => {
    if (!message) return
    setClosing(false)
    const out = window.setTimeout(() => setClosing(true), HOLD_MS)
    const gone = window.setTimeout(() => setMessage(''), HOLD_MS + EXIT_MS)
    return () => {
      window.clearTimeout(out)
      window.clearTimeout(gone)
    }
  }, [message, seq])

  if (!message) return null

  // portal 到 body：详情/弹窗也是 body 下的 z-50 浮层，挂在壳层里会被 relative z-10 夹住
  return createPortal(
    <div
      className="fixed inset-x-0 z-[80] flex justify-center px-4 pointer-events-none"
      style={{ bottom: 'calc(env(safe-area-inset-bottom, 0px) + 96px)' }}
      role="status"
      aria-live="polite"
    >
      <div
        key={seq}
        data-state={closing ? 'closing' : 'open'}
        className="vt-toast px-5 py-3 rounded-full bg-charcoal text-ivory text-sm shadow-xl animate-toast-in"
      >
        {message}
      </div>
    </div>,
    document.body,
  )
}