import { useEffect, useRef, useState } from 'react'
import AmbientBackground from './components/AmbientBackground'
import WordbookList from './pages/WordbookList'
import WordbookDetail from './pages/WordbookDetail'
import ApiKeyModal from './components/ApiKeyModal'
import ConfirmModal from './components/ConfirmModal'
import { isDirty } from './dirty'
import { withViewTransition } from './viewTransition'

export type View = { type: 'list' } | { type: 'book'; id: number }

/** 从 URL hash 解析视图：`#/book/<id>` → 书，其余 → 主页（支持刷新与直接打开） */
function parseView(): View {
  const prefix = '#/book/'
  const hash = window.location.hash
  if (!hash.startsWith(prefix)) return { type: 'list' }
  const raw = hash.slice(prefix.length).split('/')[0]
  const id = Number(raw)
  return raw !== '' && Number.isInteger(id) && id > 0 ? { type: 'book', id } : { type: 'list' }
}

/** 待确认的视图切换（有未保存更改时用 ConfirmModal 代替 window.confirm） */
type PendingLeave = { view: View; url: string }

export default function App() {
  // 视图初始化自 URL hash（刷新/直接打开 `#/book/3` 恢复书视图）
  const [view, setView] = useState<View>(parseView)
  // 当前视图引用：popstate 监听只注册一次，用 ref 读取最新视图
  const viewRef = useRef(view)
  useEffect(() => {
    viewRef.current = view
  }, [view])
  // 访问密钥弹窗：api.ts 在 401 时派发 qw:auth-required 事件
  const [authOpen, setAuthOpen] = useState(false)
  // 有未保存更改时的离开确认
  const [pendingLeave, setPendingLeave] = useState<PendingLeave | null>(null)

  useEffect(() => {
    const onAuth = () => setAuthOpen(true)
    window.addEventListener('qw:auth-required', onAuth)
    return () => window.removeEventListener('qw:auth-required', onAuth)
  }, [])

  /**
   * 视图切换（主页 ↔ 单词书）：写 URL 供前进/后退。
   * 统一走 withViewTransition（串行化 + vt-running/data-vt-entered），
   * 与页面内部的过渡不会互相 skip；视图未变时不开过渡。
   */
  function urlFor(next: View) {
    return next.type === 'list' ? window.location.pathname + window.location.search : `#/book/${next.id}`
  }

  function applyView(next: View, url: string) {
    window.history.pushState(null, '', url)
    // 整页切换：标题栏走"退出 / 落位"（html.vt-nav-motion）
    void withViewTransition(() => setView(next), { navMotion: true })
  }

  function navigate(next: View) {
    const same =
      next.type === view.type && (next.type === 'list' || (view.type === 'book' && next.id === view.id))
    if (same) return
    const url = urlFor(next)
    // 有未保存更改时用弹窗确认（不用原生 confirm：与项目视觉不一致）
    if (isDirty()) {
      setPendingLeave({ view: next, url })
      return
    }
    applyView(next, url)
  }

  // 浏览器前进/后退：按 hash 恢复视图（同样交叉淡化）
  useEffect(() => {
    const onPop = () => {
      const next = parseView()
      const current = viewRef.current
      const same =
        next.type === current.type &&
        (next.type === 'list' || (current.type === 'book' && next.id === current.id))
      // 视图没变（如关闭详情/详情内步骤回退）：交给页面内部处理，不在这里再开过渡
      if (same) return
      // 有未保存更改：先恢复当前 URL，再弹确认；确认后按目标视图切换
      if (isDirty()) {
        window.history.pushState(null, '', urlFor(current))
        setPendingLeave({ view: next, url: urlFor(next) })
        return
      }
      void withViewTransition(() => setView(next), { navMotion: true }).then(() => {
        // 浏览器返回/前进后焦点常落在 BODY：交给新页面的根节点，键盘用户不至于丢失位置
        if (document.activeElement === document.body || !document.activeElement) {
          document.querySelector<HTMLElement>('[data-page-root]')?.focus({ preventScroll: true })
        }
      })
    }
    window.addEventListener('popstate', onPop)
    return () => window.removeEventListener('popstate', onPop)
  }, [])

  return (
    <div className="min-h-screen bg-ivory font-sans text-charcoal antialiased">
      {/* 环境层挂在这里（而不是某个页面里）：主页/书页共用同一层背景，
          视图切换时它不卸载 → 转场前后是同一张静止底图，不会"换页换背景"。 */}
      <AmbientBackground />
      <div className="relative z-10">
        {view.type === 'list' ? (
          <WordbookList onOpen={(id) => navigate({ type: 'book', id })} />
        ) : (
          <WordbookDetail bookId={view.id} onBack={() => navigate({ type: 'list' })} />
        )}
      </div>
      <div className="texture-overlay" />
      {authOpen && <ApiKeyModal onClose={() => setAuthOpen(false)} />}
      {pendingLeave && (
        <ConfirmModal
          title="有未保存的更改"
          message="离开后这次编辑会丢失，确定离开吗？"
          confirmText="离开"
          onCancel={() => setPendingLeave(null)}
          onConfirm={() => {
            const target = pendingLeave
            setPendingLeave(null)
            applyView(target.view, target.url)
          }}
        />
      )}
    </div>
  )
}
