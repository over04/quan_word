import { createPortal } from 'react-dom'
import type { ReactNode } from 'react'

interface Props {
  onClose: () => void
  children: ReactNode
  /** 卡片最大宽度（tailwind 类，如 max-w-md / max-w-lg） */
  maxWidth?: string
}

/**
 * 通用弹窗：portal 渲染到 document.body，遮罩全屏覆盖。
 *
 * 不用 portal 时，若挂载点祖先带 transform（如 animate-fade-in-up 的
 * fill-mode: both），`fixed inset-0` 的包含块会被劫持，遮罩只覆盖局部。
 */
export default function Modal({ onClose, children, maxWidth = 'max-w-lg' }: Props) {
  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-charcoal/25 backdrop-blur-sm animate-fade-in"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className={`w-full ${maxWidth} max-h-[90vh] overflow-y-auto bg-white rounded-[2rem] border border-charcoal/5 shadow-2xl shadow-charcoal/15 p-8 md:p-9 animate-pop-in`}
      >
        {children}
      </div>
    </div>,
    document.body,
  )
}
