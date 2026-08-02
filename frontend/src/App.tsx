import { useEffect, useState } from 'react'
import WordbookList from './pages/WordbookList'
import WordbookDetail from './pages/WordbookDetail'
import ApiKeyModal from './components/ApiKeyModal'

export type View = { type: 'list' } | { type: 'book'; id: number }

export default function App() {
  const [view, setView] = useState<View>({ type: 'list' })
  // 访问密钥弹窗：api.ts 在 401 时派发 qw:auth-required 事件
  const [authOpen, setAuthOpen] = useState(false)

  useEffect(() => {
    const onAuth = () => setAuthOpen(true)
    window.addEventListener('qw:auth-required', onAuth)
    return () => window.removeEventListener('qw:auth-required', onAuth)
  }, [])

  return (
    <div className="min-h-screen bg-ivory font-sans text-charcoal antialiased">
      <div className="relative z-10">
        {view.type === 'list' ? (
          <WordbookList onOpen={(id) => setView({ type: 'book', id })} />
        ) : (
          <WordbookDetail bookId={view.id} onBack={() => setView({ type: 'list' })} />
        )}
      </div>
      <div className="texture-overlay" />
      {authOpen && <ApiKeyModal onClose={() => setAuthOpen(false)} />}
    </div>
  )
}
