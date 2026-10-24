import type { CSSProperties } from 'react'
import { clearDictCache } from '../dictCache'
import { clearSearchHistory } from '../searchHistory'
import { toast } from '../toast'

/** 本地数据：只保留真正会清理浏览器侧状态的入口。 */
export default function DataSettings() {
  function clearCache() {
    clearDictCache()
    toast('已清除查词缓存')
  }

  function clearHistory() {
    clearSearchHistory()
    toast('已清除搜索历史')
  }

  return (
    <ul className="divide-y divide-charcoal/8 border-y border-charcoal/8">
      <li
        className="settings-row flex min-h-14 items-center gap-3 py-2"
        style={{ '--i': 0 } as CSSProperties}
      >
        <span className="flex-1 text-sm text-charcoal">查词缓存</span>
        <button
          type="button"
          aria-label="清除查词缓存"
          onClick={clearCache}
          className="settings-clear min-h-11 rounded-full border border-charcoal/20 px-4 text-xs font-medium text-charcoal/70 hover:bg-white"
        >
          清除
        </button>
      </li>
      <li
        className="settings-row flex min-h-14 items-center gap-3 py-2"
        style={{ '--i': 1 } as CSSProperties}
      >
        <span className="flex-1 text-sm text-charcoal">搜索历史</span>
        <button
          type="button"
          aria-label="清除搜索历史"
          onClick={clearHistory}
          className="settings-clear min-h-11 rounded-full border border-charcoal/20 px-4 text-xs font-medium text-charcoal/70 hover:bg-white"
        >
          清除
        </button>
      </li>
    </ul>
  )
}
