import { useState } from 'react'
import WordbookList from './pages/WordbookList'
import WordbookDetail from './pages/WordbookDetail'

export type View = { type: 'list' } | { type: 'book'; id: number }

export default function App() {
  const [view, setView] = useState<View>({ type: 'list' })

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
    </div>
  )
}
