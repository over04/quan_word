import { useRef, useState } from 'react'
import { words, type ImportResp } from '../api'
import Modal from './Modal'
import { DownloadIcon, UploadIcon } from './Icons'

interface Props {
  bookId: number
  onClose: () => void
  /** 导入成功后调用（父级刷新列表与书信息） */
  onImported: () => void
}

const FORMAT_HINTS = [
  '第一行为表头：单词、音标、词性、释义、例句；从第二行开始填写，每行一个单词。',
  '一个单词多个义项时，在释义列用中文分号（；）或英文分号（;）分隔。',
  '义项可直接写词性前缀（如 n. 放弃），也可在词性列统一填写。',
  '支持 .csv / .xlsx / .xls / .ods 文件（WPS 请另存为 .xlsx 或 .csv）。',
  '导入为原子操作：任一行填写有误则整批不导入，并提示具体行号。',
]

/** 导入弹窗：下载模板 + 选择文件上传，展示导入结果。 */
export default function ImportModal({ bookId, onClose, onImported }: Props) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [result, setResult] = useState<ImportResp | null>(null)
  const [downloading, setDownloading] = useState<'csv' | 'xlsx' | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

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
    setResult(null)
    setBusy(true)
    try {
      setResult(await words.importFile(bookId, file))
    } catch (e) {
      setError(e instanceof Error ? e.message : '导入失败')
    } finally {
      setBusy(false)
      if (fileRef.current) fileRef.current.value = ''
    }
  }

  function handleClose() {
    if (result) onImported()
    onClose()
  }

  return (
    <Modal onClose={handleClose} maxWidth="max-w-lg">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-full bg-sage flex items-center justify-center text-white">
            <UploadIcon className="w-4 h-4" />
          </div>
          <h2 className="font-serif text-xl text-charcoal">批量导入单词</h2>
        </div>

        {result ? (
          <div className="mt-7 text-center py-6">
            <p className="font-serif text-3xl text-charcoal tabular-nums">{result.imported}</p>
            <p className="mt-2 text-sm text-charcoal/60">个单词导入成功</p>
          </div>
        ) : (
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
              {busy && <p className="mt-2 text-xs text-charcoal/45 animate-pulse">导入中…</p>}
            </div>
          </>
        )}

        {error && (
          <pre className="mt-5 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500 whitespace-pre-wrap leading-relaxed max-h-48 overflow-y-auto">
            {error}
          </pre>
        )}

        <div className="mt-8 flex justify-end gap-3">
          <button
            onClick={handleClose}
            disabled={busy}
            className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-50"
          >
            {result ? '完成' : '关闭'}
          </button>
        </div>
    </Modal>
  )
}
