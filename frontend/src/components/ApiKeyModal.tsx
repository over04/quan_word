import { useState } from 'react'
import { setApiKey, wordbooks } from '../api'
import Modal from './Modal'

interface Props {
  onClose: () => void
}

/** 访问密钥输入弹窗：API 返回 401 时由 App 挂载；验证通过后重载页面。 */
export default function ApiKeyModal({ onClose }: Props) {
  const [key, setKey] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    const k = key.trim()
    if (!k) {
      setError('请输入访问密钥')
      return
    }
    setBusy(true)
    setError('')
    setApiKey(k)
    try {
      // 用单词书列表验证密钥有效性：401 即无效
      await wordbooks.list()
      window.location.reload()
    } catch {
      setApiKey(null)
      setError('密钥无效，请检查后重试')
      setBusy(false)
    }
  }

  return (
    <Modal onClose={onClose} maxWidth="max-w-md">
      <form onSubmit={handleSubmit}>
        <h2 className="font-serif text-xl text-charcoal">需要访问密钥</h2>
        <p className="mt-2 text-sm text-charcoal/60 leading-relaxed">
          该服务已启用访问密钥保护，请输入管理员提供的密钥后继续。
        </p>

        <div className="mt-6">
          <label className="block text-xs font-medium text-charcoal/60 mb-1.5">访问密钥</label>
          <input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="请输入密钥"
            autoFocus
            className="w-full px-4 py-2.5 rounded-xl bg-white border border-charcoal/15 text-charcoal text-sm placeholder:text-charcoal/30 focus:border-clay focus:outline-none transition-colors"
          />
        </div>

        {error && (
          <p className="mt-4 px-4 py-2.5 rounded-xl bg-red-50 border border-red-200/60 text-xs text-red-500">
            {error}
          </p>
        )}

        <div className="mt-8 flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all"
          >
            取消
          </button>
          <button
            type="submit"
            disabled={busy}
            className="px-6 py-2 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-50"
          >
            {busy ? '验证中…' : '确定'}
          </button>
        </div>
      </form>
    </Modal>
  )
}
