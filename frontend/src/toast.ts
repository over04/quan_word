type Listener = (message: string) => void

let listener: Listener | null = null

/** 全局轻提示：任意组件/模块调用，由页面壳层的 <ToastHost/> 渲染。 */
export function toast(message: string) {
  listener?.(message)
}

/** ToastHost 订阅入口；返回取消订阅函数。 */
export function subscribeToast(next: Listener) {
  listener = next
  return () => {
    if (listener === next) listener = null
  }
}
