import { flushSync } from 'react-dom'
import { useEffect, useState } from 'react'
import WordbookList from './pages/WordbookList'
import WordbookDetail from './pages/WordbookDetail'
import ApiKeyModal from './components/ApiKeyModal'

export type View = { type: 'list' } | { type: 'book'; id: number }

/** 从 URL hash 解析视图：`#/book/<id>` → 书，其余 → 主页（支持刷新与直接打开） */
function parseView(): View {
  const m = window.location.hash.match(/^#\/book\/(\d+)/)
  return m ? { type: 'book', id: Number(m[1]) } : { type: 'list' }
}

export default function App() {
  // 视图初始化自 URL hash（刷新/直接打开 `#/book/3` 恢复书视图）
  const [view, setView] = useState<View>(parseView)
  // 访问密钥弹窗：api.ts 在 401 时派发 qw:auth-required 事件
  const [authOpen, setAuthOpen] = useState(false)

  useEffect(() => {
    const onAuth = () => setAuthOpen(true)
    window.addEventListener('qw:auth-required', onAuth)
    return () => window.removeEventListener('qw:auth-required', onAuth)
  }, [])

  /** 视图切换（主页 ↔ 单词书）：写 URL 供前进/后退，View Transitions 交叉淡化；不支持时直接切换 */
  function navigate(next: View) {
    const same =
      next.type === view.type && (next.type === 'list' || (view.type === 'book' && next.id === view.id))
    if (!same) {
      const url =
        next.type === 'list'
          ? window.location.pathname + window.location.search
          : `#/book/${next.id}`
      window.history.pushState(null, '', url)
    }
    if (document.startViewTransition) {
      document.startViewTransition(() => {
        // flushSync：确保新视图在快照前同步渲染（React 事件外 setState 是异步批处理的）
        flushSync(() => setView(next))
      })
    } else {
      setView(next)
    }
  }

  // 浏览器前进/后退：按 hash 恢复视图（同样交叉淡化）
  useEffect(() => {
    const onPop = () => {
      const next = parseView()
      if (document.startViewTransition) {
        document.startViewTransition(() => flushSync(() => setView(next)))
      } else {
        setView(next)
      }
    }
    window.addEventListener('popstate', onPop)
    return () => window.removeEventListener('popstate', onPop)
  }, [])

  return (
    <div className="min-h-screen bg-ivory font-sans text-charcoal antialiased">
      <div className="relative z-10">
        {view.type === 'list' ? (
          <WordbookList onOpen={(id) => navigate({ type: 'book', id })} />
        ) : (
          <WordbookDetail bookId={view.id} onBack={() => navigate({ type: 'list' })} />
        )}
      </div>
      <div className="texture-overlay" />
      {authOpen && <ApiKeyModal onClose={() => setAuthOpen(false)} />}
    </div>
  )
}
