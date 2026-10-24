import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import type { ReactNode } from 'react'
import { lockScroll } from '../scrollLock'
import AmbientBackground from './AmbientBackground'

interface Props {
  onClose: () => void
  children: ReactNode
  /** 卡片最大宽度（tailwind 类，如 max-w-md / max-w-lg） */
  maxWidth?: string
  /** 手机端全屏（桌面端仍居中卡片） */
  fullScreen?: boolean
  /**
   * 工作表布局：卡片自身为 flex 列且不内边距，
   * 由内容自己划分「固定头部 / 滚动正文 / 贴底页脚」，页脚与视口底部无缝（含 safe-area）。
   */
  sheet?: boolean
  /**
   * 页面形态：整屏纸面（无遮罩压暗、无卡片圆角阴影、无外边距），
   * 表单这类需要大量横向/纵向空间的内容不再被弹窗的尺寸上限压住。
   */
  page?: boolean
}

/** 退出动画兜底时长（毫秒）：panelOut 220ms + 余量 */
const EXIT_FALLBACK_MS = 320

/** 卡片内可聚焦控件（跳过隐藏/禁用），用于初始焦点与 Tab 循环 */
function focusables(card: HTMLElement | null): HTMLElement[] {
  if (!card) return []
  const selector =
    'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
  return [...card.querySelectorAll<HTMLElement>(selector)].filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  )
}

/** 背景 inert 的引用计数：嵌套弹窗各自挂载/卸载时不互相摘掉 */
let inertDepth = 0
function setBackgroundInert(on: boolean) {
  const root = document.getElementById('root')
  if (!root) return
  inertDepth = Math.max(0, inertDepth + (on ? 1 : -1))
  if (inertDepth > 0) root.setAttribute('inert', '')
  else root.removeAttribute('inert')
}

let titleSeq = 0

/**
 * 通用弹窗：portal 渲染到 document.body，遮罩全屏覆盖。
 *
 * 不用 portal 时，若挂载点祖先带 transform（如 animate-fade-in-up 的
 * fill-mode: both），`fixed inset-0` 的包含块会被劫持，遮罩只覆盖局部。
 *
 * 关闭时不停留「瞬间消失」：卸载前把当前遮罩与卡片克隆一份做 200ms 退出动画，
 * 真实节点立刻卸载（不阻塞业务），动画结束再移除克隆；reduced-motion 直接卸载。
 */
export default function Modal({
  onClose,
  children,
  maxWidth = 'max-w-lg',
  fullScreen = false,
  sheet = false,
  page = false,
}: Props) {
  const overlayRef = useRef<HTMLDivElement>(null)
  const cardRef = useRef<HTMLDivElement>(null)

  // 触发元素必须在「首次渲染期间」就抓住：子节点上的 autoFocus 会在 commit 阶段
  // 先把焦点抢走，等 effect 再读 document.activeElement 拿到的已经是弹窗里的输入框，
  // 关闭时那个节点已卸载 → 焦点只能落回 body（用户报的「关掉弹窗焦点丢了」）。
  const prevFocusRef = useRef<HTMLElement | null | undefined>(undefined)
  if (prevFocusRef.current === undefined) {
    prevFocusRef.current = document.activeElement as HTMLElement | null
  }

  // dialog 语义 + 焦点管理：初始焦点进卡片、背景 inert、关闭后焦点还原到触发元素
  useEffect(() => {
    const card = cardRef.current
    const previous = prevFocusRef.current ?? null
    setBackgroundInert(true)
    if (card) {
      // 用卡片里第一个标题做 aria-labelledby（各弹窗标题都是 h2）
      const heading = card.querySelector('h2, h3')
      if (heading) {
        const id = heading.id || `modal-title-${++titleSeq}`
        heading.id = id
        card.setAttribute('aria-labelledby', id)
      }
    }
    // 优先把焦点放到第一个文本控件（表单类弹窗直接可输入）；没有就退到第一个可聚焦控件
    const field = card?.querySelector<HTMLElement>('input:not([type=hidden]), textarea, select')
    const first = field ?? focusables(card)[0]
    ;(first ?? card)?.focus({ preventScroll: true })
    return () => {
      setBackgroundInert(false)
      if (previous && typeof previous.focus === 'function' && previous.isConnected) {
        previous.focus({ preventScroll: true })
      }
    }
  }, [])

  // Tab 焦点陷阱：焦点只在卡片内循环（不逃到背景）
  const trapTab = (e: React.KeyboardEvent) => {
    if (e.key !== 'Tab') return
    const card = cardRef.current
    const items = focusables(card)
    if (!card || items.length === 0) {
      e.preventDefault()
      return
    }
    const first = items[0]
    const last = items[items.length - 1]
    const active = document.activeElement
    const inside = card.contains(active)
    if (e.shiftKey) {
      if (!inside || active === first) {
        e.preventDefault()
        last.focus()
      }
    } else if (!inside || active === last) {
      e.preventDefault()
      first.focus()
    }
  }

  // Escape 统一走 onClose（编辑弹窗传入带未保存守卫的 requestClose）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  // 锁定背景滚动：引用计数（与详情页共用一个锁），
  // 嵌套弹窗/详情页异步卸载都不会把 body 永久锁在 hidden
  useEffect(() => lockScroll(), [])

  // 卸载瞬间做退出动画：克隆当前遮罩/面板并挂 motion 的 .modal-exit* 类（panelOut/overlayOut），
  // 真实节点立刻卸载（不阻塞业务），克隆层只负责视觉收尾；reduced-motion 直接卸载。
  useEffect(() => {
    // 挂载时抓住节点：卸载阶段 ref 已被清空、DOM 也可能已移除，
    // 但克隆不依赖节点是否在文档里（样式来自 class）
    const overlay = overlayRef.current
    const card = cardRef.current
    return () => {
      if (!overlay || !card) return
      if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
      const clone = overlay.cloneNode(true) as HTMLElement
      const cloneCard = clone.querySelector<HTMLElement>('[data-modal-card]')
      clone.classList.remove('animate-fade-in', 'animate-page-in')
      clone.classList.add('modal-exit-overlay')
      clone.setAttribute('aria-hidden', 'true')
      if (cloneCard) {
        // 卡片形态：撤掉入场再缩回去；页面形态（page）也补一条收尾动势，
        // 否则整页只有 180ms 淡出、纸卡僵在原地（两种形态手感不一致）。
        cloneCard.classList.remove('animate-pop-in')
        cloneCard.classList.add('modal-exit')
      }
      document.body.appendChild(clone)
      let removed = false
      const remove = () => {
        if (removed) return
        removed = true
        clone.remove()
      }
      cloneCard?.addEventListener('animationend', (event) => {
        if ((event as AnimationEvent).animationName === 'panelOut') remove()
      })
      window.setTimeout(remove, EXIT_FALLBACK_MS)
    }
  }, [])

  // 工作表在手机端铺满视口、桌面端铺满「视口 − 2rem」（遮罩已有 p-4），
  // 高度确定才能让内部滚动区正确收缩，页脚才能贴住底部
  const fullScreenCard = fullScreen
    ? sheet
      ? 'h-full rounded-none sm:rounded-[1.75rem]'
      : 'h-full sm:h-auto sm:max-h-[90vh] rounded-none sm:rounded-[1.75rem]'
    : 'max-h-[90vh] rounded-[1.75rem]'

  // 页面形态：不压暗、不漂浮，就是一整张纸面（自带环境底图，与全站连续）
  if (page) {
    return createPortal(
      <div
        ref={overlayRef}
        onKeyDown={trapTab}
        className="fixed inset-0 z-50 bg-ivory overflow-hidden animate-page-in"
      >
        <AmbientBackground inline />
        {/* role/aria-modal 必须和 aria-labelledby 落在同一个节点上：
            原来 role 在外层遮罩、aria-labelledby 在内层卡片（普通 div），
            读屏进入页面形态只报「对话框」，没有名称。 */}
        <div
          ref={cardRef}
          data-modal-card=""
          role="dialog"
          aria-modal="true"
          tabIndex={-1}
          className="relative z-10 flex h-full flex-col"
        >
          {children}
        </div>
      </div>,
      document.body,
    )
  }

  return createPortal(
    <div
      ref={overlayRef}
      className={`fixed inset-0 z-50 flex justify-center bg-charcoal/30 sm:backdrop-blur-[2px] animate-fade-in ${
        fullScreen ? 'items-stretch p-0 sm:items-center sm:p-4' : 'items-center p-4'
      }`}
      onClick={onClose}
      onKeyDown={trapTab}
    >
      <div
        ref={cardRef}
        data-modal-card=""
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        className={`w-full ${maxWidth} bg-white border border-charcoal/6 shadow-[0_40px_110px_-40px_rgb(47_42_37/0.42)] animate-pop-in ${fullScreenCard} ${
          sheet
            ? 'flex flex-col overflow-hidden'
            : 'p-6 sm:p-9 md:p-10 overflow-y-auto overscroll-contain'
        }`}
      >
        {children}
      </div>
    </div>,
    document.body,
  )
}
