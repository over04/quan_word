const STORAGE_KEY = 'qw_dict_history'
const LIMIT = 8

/** 读取最近查词（去空、去重、截断）。 */
export function loadSearchHistory(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter((item): item is string => typeof item === 'string' && item.trim() !== '')
      .slice(0, LIMIT)
  } catch {
    return []
  }
}

/** 清空最近查词。 */
export function clearSearchHistory() {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    // 忽略
  }
}

/** 记录一次查词，返回更新后的列表。 */
export function pushSearchHistory(word: string): string[] {
  const value = word.trim()
  if (!value) return loadSearchHistory()
  const next = [value, ...loadSearchHistory().filter((item) => item.toLowerCase() !== value.toLowerCase())]
    .slice(0, LIMIT)
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  } catch {
    // 隐私模式/配额不足：仅当前会话生效
  }
  return next
}
