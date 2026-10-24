import { useEffect, useState } from 'react'
import { registerDirty, unregisterDirty } from '../dirty'

/**
 * 未保存状态注册：dirty 时刷新/关闭页面触发浏览器原生提示，
 * 同时让 App 拦截浏览器后退（SPA 导航）。
 */
export function useUnsavedChanges(dirty: boolean) {
  useEffect(() => {
    if (!dirty) return
    const key = Symbol('unsaved')
    registerDirty(key)
    const onBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault()
      // 部分浏览器需要设置 returnValue 才会弹出确认
      e.returnValue = ''
    }
    window.addEventListener('beforeunload', onBeforeUnload)
    return () => {
      window.removeEventListener('beforeunload', onBeforeUnload)
      unregisterDirty(key)
    }
  }, [dirty])
}

/** 关闭守卫：dirty 时先弹确认框，否则直接执行关闭。 */
export function useDiscardGuard(dirty: boolean, onClose: () => void) {
  const [confirmOpen, setConfirmOpen] = useState(false)

  function requestClose() {
    if (dirty) setConfirmOpen(true)
    else onClose()
  }

  function confirmDiscard() {
    setConfirmOpen(false)
    onClose()
  }

  function cancelDiscard() {
    setConfirmOpen(false)
  }

  return { confirmOpen, requestClose, confirmDiscard, cancelDiscard }
}
