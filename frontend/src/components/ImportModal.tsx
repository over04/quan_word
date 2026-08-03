import { useEffect, useMemo, useRef, useState } from 'react'
import {
  words,
  type ImportPreviewResp,
  type ImportResp,
  type ImportRowData,
  type ImportRowsResp,
  type ImportRowView,
} from '../api'
import Modal from './Modal'
import Pagination from './Pagination'
import { DownloadIcon, UploadIcon } from './Icons'

interface Props {
  bookId: number
  onClose: () => void
  /** 导入成功后调用（父级刷新列表与书信息） */
  onImported: () => void
}

const FORMAT_HINTS = [
  '第一行为表头：单词、音标、词性、释义、例句、标签；从第二行开始填写。',
  '每行一个词性+释义；同一单词的多个义项写多行（单词列重复填写），导入时自动合并。',
  '标签列多个标签用分号（；）分隔，不存在的标签导入时自动创建。',
  '同书重复拼写的单词默认更新（合并标签），可在预览中改为跳过。',
  '上传后先预览：所有行可编辑修正，确认后再导入；有误的行会跳过并提示行号。',
  '支持 .csv / .xlsx / .xls / .ods 文件（WPS 请另存为 .xlsx 或 .csv）。',
]

const PAGE_SIZE = 25
const EDIT_DEBOUNCE_MS = 500

/** 词性选项：与后端白名单一致（C 可数 / U 不可数名词，vt./vi. 动词） */
const POS_OPTIONS = [
  'n.',
  'C',
  'U',
  'CU',
  'v.',
  'vt.',
  'vi.',
  'adj.',
  'adv.',
  'prep.',
  'conj.',
  'pron.',
  'num.',
  'art.',
  'interj.',
  'aux.',
  'abbr.',
  'phr.',
]

type Filter = 'all' | 'error' | 'duplicate'

/** 可编辑字段（排除只读行号） */
type EditableField = Exclude<keyof ImportRowData, 'row'>

/**
 * 导入弹窗：下载模板 → 上传预览 → 预览（后端分页/校验为主，前端仅显示与行级编辑草稿）→ 确认导入。
 *
 * 数据流：预览建立后端会话（token）；行数据/统计/错误标记全部来自后端；
 * 前端编辑只产生草稿（drafts），防抖提交 `importRows` 后由后端重新校验并返回当前页。
 */
export default function ImportModal({ bookId, onClose, onImported }: Props) {
  const [phase, setPhase] = useState<'pick' | 'preview' | 'result'>('pick')
  const [preview, setPreview] = useState<ImportPreviewResp | null>(null)
  /** 本地编辑草稿（行号 → 修正数据）；提交成功后清空 */
  const [drafts, setDrafts] = useState<Map<number, ImportRowData>>(new Map())
  /** 明确「跳过」的重复组（组首行号集合）；其余重复组默认「更新」 */
  const [skipRows, setSkipRows] = useState<Set<number>>(new Set())
  const [result, setResult] = useState<ImportResp | null>(null)
  const [filter, setFilter] = useState<Filter>('all')
  const [page, setPage] = useState(1)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [downloading, setDownloading] = useState<'csv' | 'xlsx' | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)
  const editTimer = useRef<number | null>(null)
  const reqSeq = useRef(0)

  /** 当前页按组分组（后端已按组切片，页内组完整） */
  const grouped = useMemo(() => {
    const m = new Map<number, ImportRowView[]>()
    for (const r of preview?.rows ?? []) {
      const arr = m.get(r.group) ?? []
      arr.push(r)
      m.set(r.group, arr)
    }
    return [...m.values()]
  }, [preview])

  /** 拉取当前页：提交草稿修正 → 后端重新校验 → 按筛选分页返回。immediate=false 时防抖。返回响应或 null（被取代/无会话）。 */
  async function fetchRows(
    nextPage: number,
    nextFilter: Filter,
    updates: ImportRowData[],
    immediate: boolean,
  ): Promise<ImportRowsResp | null> {
    if (editTimer.current !== null) {
      window.clearTimeout(editTimer.current)
      editTimer.current = null
    }
    const seq = ++reqSeq.current
    const run = async (): Promise<ImportRowsResp | null> => {
      if (!preview?.token) return null
      try {
        const resp = await words.importRows(bookId, {
          token: preview.token,
          page: nextPage,
          page_size: PAGE_SIZE,
          filter: nextFilter,
          updates,
        })
        if (seq !== reqSeq.current) return null // 过期响应丢弃（竞态保护）
        setPreview((d) => (d ? { ...d, ...resp } : d))
        // 仅清除本次已提交的行（保留响应期间的新输入）
        const submitted = new Set(updates.map((u) => u.row))
        setDrafts((prev) => {
          const next = new Map(prev)
          for (const r of submitted) next.delete(r)
          return next
        })
        return resp
      } catch (e) {
        if (seq !== reqSeq.current) return null
        setError(e instanceof Error ? e.message : '刷新失败')
        return null
      }
    }
    if (immediate) {
      return run()
    }
    // tsconfig lib=ES2023 无 Promise.withResolvers，用单 resolve 构造
    return new Promise<ImportRowsResp | null>((resolve) => {
      editTimer.current = window.setTimeout(() => void run().then(resolve), EDIT_DEBOUNCE_MS)
    })
  }

  /** 编辑即存草稿：防抖提交给后端重新校验 */
  function handleEdit(row: ImportRowView, field: EditableField, value: string) {
    setDrafts((prev) => {
      const next = new Map(prev)
      const base = prev.get(row.row) ?? {
        row: row.row,
        spelling: row.spelling,
        phonetic: row.phonetic,
        pos: row.pos,
        meaning: row.meaning,
        example: row.example,
        tags: row.tags,
      }
      next.set(row.row, { ...base, [field]: value })
      return next
    })
  }

  /** 编辑草稿就绪后统一防抖提交（依赖最新 drafts）；句柄写入 ref 供导航/确认取消 */
  useEffect(() => {
    if (drafts.size === 0 || !preview) return
    const timer = window.setTimeout(() => {
      void fetchRows(page, filter, [...drafts.values()], true)
    }, EDIT_DEBOUNCE_MS)
    editTimer.current = timer
    return () => {
      window.clearTimeout(timer)
      if (editTimer.current === timer) editTimer.current = null
    }
  }, [drafts]) // eslint-disable-line react-hooks/exhaustive-deps

  /** 组级字段（单词/音标/例句/标签）编辑：分发到组内所有行（与后端「首个非空/标签并集」语义一致：全部清空 = 无值） */
  function editGroupField(group: ImportRowView[], key: EditableField, value: string) {
    for (const r of group) handleEdit(r, key, value)
  }

  /** 组级字段当前值：组内首个非空（草稿优先，否则后端行值） */
  function groupValue(group: ImportRowView[], key: EditableField) {
    for (const r of group) {
      const v = drafts.get(r.row)?.[key] ?? r[key]
      if (v.trim()) return v
    }
    return ''
  }

  /** 行字段当前值（草稿优先，否则后端行值） */
  function rowValue(row: ImportRowView, key: EditableField) {
    return drafts.get(row.row)?.[key] ?? row[key]
  }

  function toggleSkip(group: number) {
    setSkipRows((prev) => {
      const next = new Set(prev)
      if (next.has(group)) next.delete(group)
      else next.add(group)
      return next
    })
  }

  function switchFilter(f: Filter) {
    setFilter(f)
    setPage(1)
    void fetchRows(1, f, [...drafts.values()], true)
  }

  function goPage(p: number) {
    setPage(p)
    void fetchRows(p, filter, [...drafts.values()], true)
  }

  async function handleDownload(format: 'csv' | 'xlsx') {
    setDownloading(format)
    setError('')
    try {
      await words.downloadTemplate(bookId, format)
    } catch (e) {
      setError(e instanceof Error ? e.message : '下载失败')
    } finally {
      setDownloading(null)
    }
  }

  async function handleFile(file: File) {
    setError('')
    setBusy(true)
    reqSeq.current++ // 使上一会话的在途响应失效，防止新预览被旧响应污染
    try {
      const resp = await words.importPreview(bookId, file, 1, PAGE_SIZE)
      setPreview(resp)
      setDrafts(new Map())
      setSkipRows(new Set())
      setFilter('all')
      setPage(1)
      setPhase('preview')
    } catch (e) {
      setError(e instanceof Error ? e.message : '预览失败')
    } finally {
      setBusy(false)
      if (fileRef.current) fileRef.current.value = ''
    }
  }

  async function handleConfirm() {
    if (!preview) return
    setBusy(true)
    setError('')
    try {
      // 冲洗未提交草稿：确保会话包含最后修正（防抖窗口内确认不再丢修正）
      let dupGroups = preview.duplicate_groups
      if (drafts.size > 0 || editTimer.current !== null) {
        const resp = await fetchRows(page, filter, [...drafts.values()], true)
        if (resp) dupGroups = resp.duplicate_groups
      }
      // 更新集合 = 全部重复组 − 用户跳过的组
      const updateRows = dupGroups.filter((g) => !skipRows.has(g))
      setResult(await words.importFile(bookId, preview.token, updateRows))
      setPhase('result')
    } catch (e) {
      setError(e instanceof Error ? e.message : '导入失败')
    } finally {
      setBusy(false)
    }
  }

  function handleClose() {
    if (result) onImported()
    onClose()
  }

  function backToPick() {
    setPreview(null)
    setDrafts(new Map())
    setSkipRows(new Set())
    setError('')
    setPhase('pick')
  }

  const tabClass = (active: boolean) =>
    `px-3 py-1.5 rounded-full text-xs font-medium transition-colors ${
      active ? 'bg-charcoal text-ivory' : 'bg-sand/50 text-charcoal/70 hover:bg-sand/80'
    }`

  const inputClass =
    'min-w-0 px-2 py-1 rounded-md bg-white border border-charcoal/10 text-sm text-charcoal focus:border-clay focus:outline-none transition-colors'
  const labelClass = 'shrink-0 text-[10px] text-charcoal/40 w-7'

  return (
    <Modal onClose={handleClose} maxWidth="max-w-5xl">
      <div className="flex items-center gap-3">
        <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
          <UploadIcon className="w-4 h-4" />
        </div>
        <h2 className="font-serif text-xl text-charcoal">
          {phase === 'pick' && '批量导入单词'}
          {phase === 'preview' && '导入预览'}
          {phase === 'result' && '导入结果'}
        </h2>
      </div>

      {phase === 'pick' && (
        <>
          <ul className="mt-6 space-y-2">
            {FORMAT_HINTS.map((h) => (
              <li key={h} className="flex gap-2 text-xs text-charcoal/60 leading-relaxed">
                <span className="shrink-0 text-clay mt-0.5">·</span>
                <span>{h}</span>
              </li>
            ))}
          </ul>

          <div className="mt-6 flex flex-wrap gap-3">
            <button
              onClick={() => handleDownload('csv')}
              disabled={downloading !== null}
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-full border border-charcoal/15 text-sm font-medium text-charcoal/80 hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
            >
              <DownloadIcon className="w-4 h-4" />
              {downloading === 'csv' ? '下载中…' : '下载 CSV 模板'}
            </button>
            <button
              onClick={() => handleDownload('xlsx')}
              disabled={downloading !== null}
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-full border border-charcoal/15 text-sm font-medium text-charcoal/80 hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
            >
              <DownloadIcon className="w-4 h-4" />
              {downloading === 'xlsx' ? '下载中…' : '下载 Excel 模板'}
            </button>
          </div>

          <div className="mt-6">
            <label className="block text-xs font-medium text-charcoal/60 mb-1.5">选择文件</label>
            <input
              ref={fileRef}
              type="file"
              accept=".csv,.xlsx,.xls,.ods"
              disabled={busy}
              onChange={(e) => {
                const f = e.target.files?.[0]
                if (f) handleFile(f)
              }}
              className="block w-full text-sm text-charcoal/70 file:mr-4 file:px-5 file:py-2.5 file:rounded-full file:border-0 file:bg-charcoal file:text-ivory file:text-sm file:font-medium file:cursor-pointer hover:file:bg-charcoal/90 transition-all"
            />
            {busy && <p className="mt-2 text-xs text-charcoal/45 animate-pulse">解析中…</p>}
          </div>
        </>
      )}

      {phase === 'preview' && preview && (
        <>
          <div className="mt-6 flex flex-wrap items-center gap-3">
            <p className="text-sm text-charcoal/70 tabular-nums">
              共 <span className="font-semibold text-charcoal">{preview.total_rows}</span> 行 · 可导入{' '}
              <span className="font-semibold text-charcoal">{preview.valid_rows}</span> 行 · 错误{' '}
              <span className="font-semibold text-red-500">{preview.invalid_rows}</span> 行
            </p>
            {preview.duplicate_total > 0 && (
              <div className="flex items-center gap-1.5">
                <button
                  onClick={() => setSkipRows(new Set())}
                  className="px-3 py-1 rounded-full border border-charcoal/15 text-xs text-charcoal/70 hover:bg-white transition-colors"
                >
                  全部更新
                </button>
                <button
                  onClick={() => setSkipRows(new Set(preview.duplicate_groups))}
                  className="px-3 py-1 rounded-full border border-charcoal/15 text-xs text-charcoal/70 hover:bg-white transition-colors"
                >
                  全部跳过
                </button>
              </div>
            )}
          </div>

          <div className="mt-4 flex gap-2">
            <button onClick={() => switchFilter('all')} className={tabClass(filter === 'all')}>
              全部（{preview.total_rows}）
            </button>
            <button onClick={() => switchFilter('error')} className={tabClass(filter === 'error')}>
              错误（{preview.invalid_rows}）
            </button>
            <button onClick={() => switchFilter('duplicate')} className={tabClass(filter === 'duplicate')}>
              重复（{preview.duplicate_total}）
            </button>
          </div>

          <div className="mt-4 space-y-2">
            {grouped.length === 0 ? (
              <p className="py-8 text-center text-xs text-charcoal/40">该筛选下没有行</p>
            ) : (
              grouped.map((group) => {
                const first = group[0]
                const last = group[group.length - 1]
                const edited = group.some((r) => drafts.has(r.row))
                const rowRange = first.row === last.row ? `行${first.row}` : `行${first.row}-${last.row}`
                return (
                  <div
                    key={first.group}
                    className={`rounded-xl border px-3 py-2 ${
                      first.is_duplicate ? 'border-clay/40' : 'border-charcoal/10'
                    }`}
                  >
                    {/* 组头：单词级字段（每组仅一份） */}
                    <div className="flex items-center gap-2">
                      <span className="w-10 shrink-0 text-xs text-charcoal/40 tabular-nums">
                        {rowRange}
                      </span>
                      <label className="flex items-center gap-1">
                        <span className={labelClass}>单词</span>
                        <input
                          value={groupValue(group, 'spelling')}
                          onChange={(e) => editGroupField(group, 'spelling', e.target.value)}
                          className={`${inputClass} w-36 shrink-0`}
                          aria-label={`${rowRange} 单词`}
                        />
                      </label>
                      <label className="flex items-center gap-1">
                        <span className={labelClass}>音标</span>
                        <input
                          value={groupValue(group, 'phonetic')}
                          onChange={(e) => editGroupField(group, 'phonetic', e.target.value)}
                          className={`${inputClass} w-28 shrink-0`}
                          aria-label={`${rowRange} 音标`}
                        />
                      </label>
                      <label className="flex items-center gap-1 shrink-0">
                        <span className={labelClass}>标签</span>
                        <input
                          value={groupValue(group, 'tags')}
                          onChange={(e) => editGroupField(group, 'tags', e.target.value)}
                          className={`${inputClass} w-32`}
                          aria-label={`${rowRange} 标签`}
                        />
                      </label>
                      <div className="flex-1" />
                      <span className="shrink-0 text-xs text-charcoal/40 w-12 text-right">
                        {edited ? <span className="text-clay">已修改</span> : '通过'}
                      </span>
                      {first.is_duplicate && (
                        <label className="shrink-0 inline-flex items-center gap-1 text-xs text-charcoal/70 cursor-pointer">
                          <input
                            type="checkbox"
                            checked={!skipRows.has(first.group)}
                            onChange={() => toggleSkip(first.group)}
                            className="accent-charcoal"
                          />
                          更新
                        </label>
                      )}
                    </div>
                    {/* 例句：独立一行全宽 */}
                    <div className="mt-1.5 flex items-center gap-2">
                      <span className="w-10 shrink-0" />
                      <label className="flex-1 min-w-0 flex items-center gap-1">
                        <span className={labelClass}>例句</span>
                        <input
                          value={groupValue(group, 'example')}
                          onChange={(e) => editGroupField(group, 'example', e.target.value)}
                          className={`${inputClass} flex-1 min-w-0`}
                          aria-label={`${rowRange} 例句`}
                        />
                      </label>
                    </div>
                    {/* 义项行：每个词性 + 释义一行（词性为下拉，错误标记来自后端校验） */}
                    {group.map((row) => {
                      const curPos = rowValue(row, 'pos')
                      return (
                        <div
                          key={row.row}
                          className={`flex items-center gap-2 px-3 py-1.5 rounded-lg border-t border-charcoal/5 first:border-t-0 ${
                            row.error ? 'bg-red-50/40' : ''
                          }`}
                        >
                          <span className="w-10 shrink-0 text-xs text-charcoal/40 tabular-nums">
                            {row.row}
                          </span>
                          <label className="flex items-center gap-1 shrink-0">
                            <span className={labelClass}>词性</span>
                            <select
                              value={curPos}
                              onChange={(e) => handleEdit(row, 'pos', e.target.value)}
                              className={`${inputClass} w-24`}
                              aria-label={`${rowRange} 词性`}
                            >
                              <option value="">留空</option>
                              {curPos && !POS_OPTIONS.includes(curPos) && (
                                <option value={curPos}>{curPos}</option>
                              )}
                              {POS_OPTIONS.map((p) => (
                                <option key={p} value={p}>
                                  {p}
                                </option>
                              ))}
                            </select>
                          </label>
                          <label className="flex-1 min-w-0 flex items-center gap-1">
                            <span className={labelClass}>释义</span>
                            <input
                              value={rowValue(row, 'meaning')}
                              onChange={(e) => handleEdit(row, 'meaning', e.target.value)}
                              className={`${inputClass} flex-1 min-w-0`}
                              aria-label={`${rowRange} 释义`}
                            />
                          </label>
                          {row.error && (
                            <span
                              className="shrink-0 text-xs text-red-500 max-w-64 truncate"
                              title={row.error}
                            >
                              {row.error}
                            </span>
                          )}
                        </div>
                      )
                    })}
                  </div>
                )
              })
            )}
          </div>

          {preview.total_pages > 1 && (
            <div className="mt-3">
              <Pagination
                page={page}
                totalPages={preview.total_pages}
                onPrev={() => goPage(Math.max(1, page - 1))}
                onNext={() => goPage(Math.min(preview.total_pages, page + 1))}
              />
            </div>
          )}

          <div className="mt-4">
            {preview.new_tags.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="text-xs text-charcoal/60">将自动新建标签：</span>
                {preview.new_tags.map((t) => (
                  <span
                    key={t}
                    className="px-3 py-1 rounded-full bg-sand/50 text-charcoal/70 text-xs font-medium"
                  >
                    {t}
                  </span>
                ))}
              </div>
            )}
            {preview.existing_tags > 0 && (
              <p className="mt-1.5 text-xs text-charcoal/45">
                另有 {preview.existing_tags} 个已存在的标签被引用
              </p>
            )}
          </div>
        </>
      )}

      {phase === 'result' && result && (
        <div className="mt-7 text-center py-6">
          <div className="flex items-center justify-center gap-6">
            <div>
              <p className="font-serif text-3xl text-charcoal tabular-nums">{result.imported}</p>
              <p className="mt-1 text-sm text-charcoal/60">新增</p>
            </div>
            <div>
              <p className="font-serif text-3xl text-charcoal tabular-nums">{result.updated}</p>
              <p className="mt-1 text-sm text-charcoal/60">更新</p>
            </div>
            <div>
              <p className="font-serif text-3xl text-charcoal/50 tabular-nums">
                {result.skipped_errors + result.skipped_duplicates}
              </p>
              <p className="mt-1 text-sm text-charcoal/60">跳过</p>
            </div>
          </div>
          {result.created_tags > 0 && (
            <p className="mt-4 text-sm text-charcoal/60">新建标签 {result.created_tags} 个</p>
          )}
        </div>
      )}

      {error && (
        <pre className="mt-5 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500 whitespace-pre-wrap leading-relaxed max-h-48 overflow-y-auto">
          {error}
        </pre>
      )}

      <div className="mt-8 flex justify-end gap-3">
        {phase === 'preview' && (
          <>
            <button
              onClick={backToPick}
              disabled={busy}
              className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
            >
              返回
            </button>
            <button
              onClick={handleConfirm}
              disabled={busy || preview?.valid_rows === 0}
              className="px-5 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all disabled:opacity-50"
            >
              {busy ? '导入中…' : '确认导入'}
            </button>
          </>
        )}
        {phase !== 'preview' && (
          <button
            onClick={handleClose}
            disabled={busy}
            className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
          >
            {result ? '完成' : '关闭'}
          </button>
        )}
      </div>
    </Modal>
  )
}
