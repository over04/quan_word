import type { DictEntry, DictProviderResp } from './api'

/** 来源下拉的一行：id + 完整名 + 短名 + 本次是否有结果。 */
export interface DictSourceRow {
  id: string
  label: string
  short: string
  available: boolean
}

/**
 * 由「已安装插件」与「本次查询结果」拼出来源下拉。
 *
 * - 只列配置启用（enabled）的插件，顺序 = 后端注册表顺序；
 * - 结果里出现但插件列表没来得及刷新的源追加在后面（插件热重载后仍可切换）；
 * - available 表示本次查询该源返回了词条（无结果的源在下拉里灰掉并标「无结果」）。
 */
export function buildSourceRows(
  providers: DictProviderResp[],
  entries: DictEntry[],
): DictSourceRow[] {
  const rows = new Map<string, DictSourceRow>()
  for (const provider of providers) {
    if (!provider.enabled) continue
    rows.set(provider.id, {
      id: provider.id,
      label: provider.label,
      short: provider.short_label || provider.label,
      available: false,
    })
  }
  for (const entry of entries) {
    const row = rows.get(entry.provider)
    if (row) {
      row.available = true
    } else {
      rows.set(entry.provider, {
        id: entry.provider,
        label: entry.label,
        short: entry.label,
        available: true,
      })
    }
  }
  return Array.from(rows.values())
}
