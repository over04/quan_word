/** 背景滚动锁：引用计数，避免多个浮层（详情/弹窗）叠开时互相覆盖、关完仍锁着。 */
let depth = 0
let saved = ''

/** 上锁；返回释放函数（重复调用安全）。 */
export function lockScroll(): () => void {
  if (depth === 0) {
    saved = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
  depth += 1
  let released = false
  return () => {
    if (released) return
    released = true
    depth = Math.max(0, depth - 1)
    if (depth === 0) document.body.style.overflow = saved
  }
}
