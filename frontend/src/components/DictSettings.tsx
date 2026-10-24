import { useEffect, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { dict, type DictPluginDetailResp, type DictProviderResp } from '../api'
import { clearSuggestCache } from '../hooks/useDictSuggest'
import { toast } from '../toast'
import DictPluginDetail from './DictPluginDetail'
import { BookIcon, ChevronRightIcon, SparkIcon, TrashIcon, UploadIcon } from './Icons'

/**
 * 词典源设置：列表、启停、导入、删除。
 * 插件文件由服务端目录自动发现；这里不展示目录、报告或统计。
 */
export default function DictSettings() {
  const [plugins, setPlugins] = useState<DictProviderResp[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [pending, setPending] = useState<string | null>(null)
  const [selected, setSelected] = useState<DictProviderResp | null>(null)
  const [detail, setDetail] = useState<DictPluginDetailResp | null>(null)
  const [detailLoading, setDetailLoading] = useState(false)
  const [detailError, setDetailError] = useState('')
  const [detailReload, setDetailReload] = useState(0)
  const alive = useRef(true)
  const loadSeq = useRef(0)

  async function load(silent = false) {
    const seq = ++loadSeq.current
    if (!silent) setLoading(true)
    try {
      const list = await dict.providers()
      if (!alive.current || seq !== loadSeq.current) return
      setPlugins(list)
      setError('')
    } catch (e) {
      if (alive.current && seq === loadSeq.current) {
        setError(e instanceof Error ? e.message : '加载失败')
      }
    } finally {
      if (alive.current && seq === loadSeq.current && !silent) setLoading(false)
    }
  }

  useEffect(() => {
    alive.current = true
    void load()
    const onFocus = () => void load(true)
    const onVisible = () => {
      if (document.visibilityState === 'visible') void load(true)
    }
    window.addEventListener('focus', onFocus)
    document.addEventListener('visibilitychange', onVisible)
    return () => {
      alive.current = false
      window.removeEventListener('focus', onFocus)
      document.removeEventListener('visibilitychange', onVisible)
    }
    // 首次进入设置分区拉一次；之后靠窗口重新聚焦/页面可见时静默同步
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  useEffect(() => {
    if (!selected) {
      setDetail(null)
      setDetailError('')
      return
    }
    let active = true
    setDetailLoading(true)
    setDetailError('')
    dict
      .pluginDetail(selected.id)
      .then((value) => {
        if (active) setDetail(value)
      })
      .catch((e: unknown) => {
        if (active) setDetailError(e instanceof Error ? e.message : '详情加载失败')
      })
      .finally(() => {
        if (active) setDetailLoading(false)
      })
    return () => {
      active = false
    }
  }, [selected, detailReload])

  useEffect(() => {
    if (!selected) return
    const current = plugins.find((plugin) => plugin.id === selected.id)
    if (!current) {
      if (!loading) setSelected(null)
      return
    }
    if (current.enabled !== selected.enabled || current.version !== selected.version) {
      setSelected(current)
    }
  }, [plugins, loading, selected])

  async function toggle(plugin: DictProviderResp) {
    if (pending) return
    loadSeq.current += 1
    setPending(plugin.id)
    try {
      const updated = await dict.setPluginEnabled(plugin.id, !plugin.enabled)
      clearSuggestCache()
      setPlugins((current) => current.map((item) => (item.id === updated.id ? updated : item)))
    } catch (e) {
      toast(e instanceof Error ? e.message : '操作失败')
    } finally {
      setPending(null)
    }
  }

  async function importFile(file: File) {
    if (pending) return
    loadSeq.current += 1
    setPending('import')
    try {
      const list = await dict.importPlugin(file)
      clearSuggestCache()
      setPlugins(list)
      setError('')
      toast('已导入')
    } catch (e) {
      toast(e instanceof Error ? e.message : '导入失败')
    } finally {
      setPending(null)
    }
  }

  async function remove(plugin: DictProviderResp) {
    if (pending) return
    if (!window.confirm(`删除「${plugin.label}」？`)) return
    loadSeq.current += 1
    setPending(plugin.id)
    try {
      const list = await dict.deletePlugin(plugin.id)
      clearSuggestCache()
      setPlugins(list)
      setError('')
      toast('已删除')
    } catch (e) {
      toast(e instanceof Error ? e.message : '删除失败')
    } finally {
      setPending(null)
    }
  }

  if (loading) {
    return (
      <div className="space-y-2" aria-label="正在加载词典源">
        {[0, 1, 2, 3].map((index) => (
          <div key={index} className="h-14 rounded-xl bg-sand/20 animate-pulse" />
        ))}
      </div>
    )
  }

  if (selected) {
    return (
      <DictPluginDetail
        plugin={selected}
        detail={detail}
        loading={detailLoading}
        error={detailError}
        onBack={() => setSelected(null)}
        onRetry={() => setDetailReload((value) => value + 1)}
      />
    )
  }

  return (
    <div>
      {error && <p className="mb-3 text-xs text-red-700">{error}</p>}
      {plugins.length === 0 && (
        <div className="settings-empty" aria-hidden="true">
          <span className="settings-empty-ring" />
          <span className="settings-empty-ring settings-empty-ring--inner" />
          <span className="settings-empty-sweep" />
          <span className="settings-empty-dot" />
          <span className="settings-empty-core">
            <UploadIcon className="settings-empty-icon" />
          </span>
        </div>
      )}
      <ul
        className={`settings-list relative divide-y divide-charcoal/8 border-y border-charcoal/8 ${
          plugins.length === 0 ? 'settings-list--empty' : ''
        }`}
      >
        {plugins.map((plugin, index) => (
          <li
            key={plugin.id}
            className={`settings-row flex min-h-14 items-center gap-2 ${plugin.enabled ? 'is-on' : ''}`}
            style={{ '--i': index } as CSSProperties}
          >
            <button
              type="button"
              className="settings-row-main"
              onClick={() => setSelected(plugin)}
              aria-label={`查看 ${plugin.label} 详情`}
            >
              <span className="settings-row-pin" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate font-serif text-base text-charcoal" title={plugin.id}>
                {plugin.label}
              </span>
              <span className="settings-row-caps" aria-hidden="true">
                {plugin.capabilities.includes('lookup') && <BookIcon className="h-3.5 w-3.5" />}
                {plugin.capabilities.includes('suggest') && <SparkIcon className="h-3.5 w-3.5" />}
              </span>
              <span className="settings-row-version">v{plugin.version}</span>
              <ChevronRightIcon className="settings-row-chevron h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              role="switch"
              aria-checked={plugin.enabled}
              aria-label={plugin.label}
              disabled={pending === plugin.id}
              onClick={() => void toggle(plugin)}
              className="settings-switch flex h-11 w-11 shrink-0 items-center justify-center disabled:opacity-50"
            >
              <span
                aria-hidden="true"
                className={`settings-switch-track relative h-6 w-11 rounded-full ${
                  plugin.enabled ? 'bg-charcoal' : 'bg-charcoal/15'
                }`}
              >
                <span
                  className={`settings-switch-knob absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow-sm ${
                    plugin.enabled ? 'translate-x-5' : 'translate-x-0'
                  }`}
                />
              </span>
            </button>
            <button
              type="button"
              aria-label={`删除 ${plugin.label}`}
              disabled={pending === plugin.id}
              onClick={() => void remove(plugin)}
              className="settings-delete flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-charcoal/35 hover:bg-red-50 hover:text-red-700 disabled:opacity-50"
            >
              <TrashIcon className="h-4 w-4" />
            </button>
          </li>
        ))}
        <li className="settings-row flex min-h-14 items-center py-2" style={{ '--i': plugins.length } as CSSProperties}>
          <span className="settings-row-pin settings-row-pin--add" aria-hidden="true" />
          <label
            className={`settings-import relative flex min-h-11 cursor-pointer items-center gap-2 px-3 text-sm text-charcoal/55 hover:text-charcoal ${
              pending === 'import' ? 'pointer-events-none opacity-50' : ''
            }`}
          >
            <UploadIcon className="h-4 w-4" />
            导入插件
            <input
              type="file"
              accept=".json,application/json"
              className="sr-only"
              aria-label="导入插件"
              onChange={(event) => {
                const file = event.currentTarget.files?.[0]
                event.currentTarget.value = ''
                if (file) void importFile(file)
              }}
            />
          </label>
        </li>
      </ul>
    </div>
  )
}
