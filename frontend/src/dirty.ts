/**
 * 未保存更改的全局注册表。
 *
 * 编辑弹窗在存在未保存草稿时注册一个 key，App 在浏览器后退（popstate）
 * 时通过 isDirty() 拦截导航；弹窗卸载或保存后注销。
 */
const keys = new Set<symbol>()

/** 注册一个未保存编辑器（重复注册同一 key 无副作用）。 */
export function registerDirty(key: symbol) {
  keys.add(key)
}

/** 注销未保存编辑器。 */
export function unregisterDirty(key: symbol) {
  keys.delete(key)
}

/** 当前是否存在未保存更改。 */
export function isDirty(): boolean {
  return keys.size > 0
}
