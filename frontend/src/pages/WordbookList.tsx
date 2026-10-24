import { useCallback, useEffect, useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import { words, wordbooks, type Definition, type DictEntry, type Wordbook } from '../api'
import Bookshelf from '../components/Bookshelf'
import ConfirmModal from '../components/ConfirmModal'
import DictDetailPage from '../components/DictDetailPage'
import DictHome from '../components/DictHome'
import HomeNav, { type HomeTab } from '../components/HomeNav'
import { RefreshIcon } from '../components/Icons'
import ToastHost from '../components/Toast'
import { toast } from '../toast'
import WordbookFormModal from '../components/WordbookFormModal'
import { exampleText } from '../hooks/useDictFill'
import { clearPresenceCache } from '../hooks/useWordPresence'
import { setTransitionWord, withViewTransition } from '../viewTransition'

interface Props {
  onOpen: (id: number) => void
}

/** 主页壳层：一级分区、书架数据、详情/加入流程、弹窗与 toast。 */
export default function WordbookList({ onOpen }: Props) {
  const [books, setBooks] = useState<Wordbook[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [tab, setTab] = useState<HomeTab>('dict')
  const [formOpen, setFormOpen] = useState(false)
  const [editing, setEditing] = useState<Wordbook | null>(null)
  const [pendingDelete, setPendingDelete] = useState<Wordbook | null>(null)
  const [detailWord, setDetailWord] = useState<string | null>(null)
  const [presenceVersion, setPresenceVersion] = useState(0)
  // 详情是否已压入历史条目：同步标记，快速连点/返回时 state 尚未更新也能判断
  const detailOpenRef = useRef(false)
  // 详情内部的「后退一步」实现（浏览器后退与 Esc 共用）
  const detailBackRef = useRef<(() => boolean) | null>(null)
  // 从详情里点「创建单词书」时记住当前词，建完书回到该词继续加入
  const pendingWordRef = useRef<string | null>(null)
  // 仅用于在开转场前强制渲染一次（让被点词先挂上 view-transition-name）
  const [, setVtTick] = useState(0)

  const registerDetailBack = useCallback((handler: (() => boolean) | null) => {
    detailBackRef.current = handler
  }, [])

  async function refresh() {
    try {
      setError('')
      setBooks(await wordbooks.list())
    } catch (e) {
      setError(e instanceof Error ? e.message : '加载失败')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void refresh()
    window.scrollTo(0, 0)
  }, [])

  // 浏览器返回键：先按详情内部层级退步（definitions → books → detail），
  // 到详情首页再关闭；避免在「选择单词书/选择释义」里直接退出整个详情丢词。
  useEffect(() => {
    const onPop = () => {
      if (!detailOpenRef.current) return
      const stepBack = detailBackRef.current
      if (stepBack?.()) {
        // 步骤内已消费：补回历史条目，下一次后退继续按层级退
        window.history.pushState({ qwDict: true }, '')
        return
      }
      detailOpenRef.current = false
      // App 的 popstate 在视图未变时不开过渡，这里自己跑关闭动画
      void withViewTransition(() => setDetailWord(null), { navMotion: true })
    }
    window.addEventListener('popstate', onPop)
    return () => window.removeEventListener('popstate', onPop)
  }, [])

  function switchTab(next: HomeTab) {
    if (next === tab) return
    void withViewTransition(() => setTab(next))
  }

  function openDetail(word: string) {
    if (!detailOpenRef.current) {
      detailOpenRef.current = true
      window.history.pushState({ qwDict: true }, '')
    }
    // 共享元素命名必须出现在「旧快照」里：模块变量不触发渲染，
    // 这里先同步渲染一次（flushSync），再开转场
    setTransitionWord(word)
    flushSync(() => setVtTick((n) => n + 1))
    void withViewTransition(() => setDetailWord(word), { navMotion: true })
  }

  /**
   * 关闭详情：先跑自己的关闭过渡，结束后再回收 push 出来的历史条目。
   * 直接 history.back() 会同时触发 App 的 popstate 过渡，两条过渡互相 skip。
   */
  function closeDetail(): Promise<void> {
    // 反向共享元素：让标题飞回列表里那张卡片（列表项的名字由 transitionNameFor 决定）
    if (detailWord) setTransitionWord(detailWord)
    // 入场动画由 motion 的 html.vt-running 规则统一快进到终态（不要在这里 animate-none：
    // 销毁动画会在类名摘掉后从 0 重建，反而二次入场）
    // 关闭完成后清掉命名并重渲染一次：否则列表里那个词会一直挂着 view-transition-name
    const done = () => {
      setTransitionWord(null)
      setVtTick((n) => n + 1)
    }
    if (!detailOpenRef.current) {
      return withViewTransition(() => setDetailWord(null), { navMotion: true }).then(done)
    }
    detailOpenRef.current = false
    return withViewTransition(() => setDetailWord(null), { navMotion: true }).then(() => {
      // 期间若已经重新打开详情（如创建词书后回到该词），不要误 pop 新条目
      if (!detailOpenRef.current && window.history.state?.qwDict) window.history.back()
      done()
    })
  }

  async function saveDictWord(book: Wordbook, entry: DictEntry, definitions: Definition[]) {
    await words.create(book.id, {
      spelling: entry.word,
      phonetic: entry.phonetic ?? '',
      definitions,
      example: exampleText(entry),
      tags: [],
    })
    clearPresenceCache()
    setPresenceVersion((version) => version + 1)
    await refresh()
    // 先跑完详情关闭转场再 toast：转场期间快照覆盖整屏，提前 toast 会被吞掉
    await closeDetail()
    toast(`已加入《${book.name}》`)
  }

  async function confirmDelete() {
    if (!pendingDelete) return
    const book = pendingDelete
    setPendingDelete(null)
    try {
      await wordbooks.remove(book.id)
      toast(`已删除《${book.name}》`)
      await refresh()
    } catch (e) {
      setError(e instanceof Error ? e.message : '删除失败')
    }
  }

  return (
    <div className="relative min-h-screen">
      {/* 详情打开时卸载首页导航：它不在 .vt-home 里，留着会在转场快照中叠在详情页之上
          （导航分组 z-index 3 > detail-page 1），顶部同时出现两条导航。 */}
      {!detailWord && (
        <div className="relative z-40">
          <HomeNav
            tab={tab}
            onTab={switchTab}
            onCreate={() => {
              setEditing(null)
              setFormOpen(true)
            }}
          />
        </div>
      )}

      <main className="relative z-10 pt-[calc(env(safe-area-inset-top,0px)+10rem)] md:pt-[calc(env(safe-area-inset-top,0px)+7rem)] pb-16">
        {error && (
          <div className="text-center mb-4 px-4">
            <p className="text-red-600">{error}</p>
            <button
              type="button"
              onClick={() => void refresh()}
              className="mt-3 inline-flex items-center gap-1.5 min-h-11 px-5 rounded-full border border-charcoal/20 text-sm font-medium text-charcoal/75 hover:bg-charcoal/5"
            >
              <RefreshIcon className="w-3.5 h-3.5" />
              重试
            </button>
          </div>
        )}
        <div className={`vt-home outline-none ${detailWord ? 'hidden' : ''}`} data-page-root="" tabIndex={-1}>
          {tab === 'dict' ? (
            <DictHome presenceVersion={presenceVersion} onOpenDetail={openDetail} />
          ) : (
            <section className="max-w-7xl mx-auto px-4 md:px-8 animate-page-in" id="books">
              <Bookshelf
                books={books}
                loading={loading}
                onOpen={onOpen}
                onCreate={() => {
                  setEditing(null)
                  setFormOpen(true)
                }}
                onEdit={(book) => {
                  setEditing(book)
                  setFormOpen(true)
                }}
                onDelete={setPendingDelete}
              />
            </section>
          )}
        </div>
      </main>

      {formOpen && (
        <WordbookFormModal
          initial={editing}
          onClose={() => {
            setFormOpen(false)
            const pending = pendingWordRef.current
            pendingWordRef.current = null
            // 取消建书：回到刚才那个词的详情，不把用户扔回首页
            if (pending) openDetail(pending)
          }}
          onSaved={async () => {
            setFormOpen(false)
            await refresh()
            const pending = pendingWordRef.current
            pendingWordRef.current = null
            toast(editing ? '已保存单词书' : '已创建单词书')
            // 从详情里来建书：建完回到刚才那个词的详情，继续「加入词书」
            if (pending) openDetail(pending)
          }}
        />
      )}

      {pendingDelete && (
        <ConfirmModal
          title={`删除「${pendingDelete.name}」？`}
          message="其中的单词也会一并删除，此操作不可恢复。"
          onConfirm={confirmDelete}
          onCancel={() => setPendingDelete(null)}
        />
      )}

      {detailWord && (
        <DictDetailPage
          word={detailWord}
          allBooks={books}
          presenceVersion={presenceVersion}
          onSave={saveDictWord}
          onCreateBook={() => {
            pendingWordRef.current = detailWord
            closeDetail()
            setEditing(null)
            setFormOpen(true)
          }}
          onClose={closeDetail}
          onRegisterBack={registerDetailBack}
        />
      )}

      <ToastHost />
    </div>
  )
}
