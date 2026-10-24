import { BookIcon, PlusIcon } from './Icons'

export type HomeTab = 'dict' | 'books'

interface Props {
  tab: HomeTab
  onTab: (tab: HomeTab) => void
  onCreate: () => void
}

const TABS: Array<{ id: HomeTab; label: string }> = [
  { id: 'dict', label: '查词' },
  { id: 'books', label: '单词书' },
]

/**
 * 主页一级导航：品牌 / 查词与单词书分区 / 新建。
 *
 * 动效与性能：
 * - 入场只用 transform + opacity（animate-nav-in），动画结束后不留合成层；
 * - 分区用分段控件，白色滑块靠 transform 位移（.nav-seg-pill），切换只有一次合成层动画；
 * - 不用 backdrop-filter：固定导航在滚动时每帧都要重算背景模糊，这里用高不透明度白色 + 内高光还原玻璃感；
 * - 所有可点区域 ≥ 44px（触控标准），移动端两行布局总高 141px < 内容顶部预留 160px，不遮挡内容。
 */
export default function HomeNav({ tab, onTab, onCreate }: Props) {
  const activeIndex = Math.max(0, TABS.findIndex((item) => item.id === tab))

  /** 品牌区＝回首页：切回查词分区并回到顶部（用 button 而不是 href="#"，
      避免往 history.state 里塞一条无意义记录）。 */
  function goHome() {
    onTab('dict')
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    window.scrollTo({ top: 0, behavior: reduce ? 'auto' : 'smooth' })
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    event.preventDefault()
    const next = (activeIndex + (event.key === 'ArrowRight' ? 1 : TABS.length - 1)) % TABS.length
    onTab(TABS[next].id)
  }

  return (
    <nav className="home-nav vt-nav-home fixed w-full top-0 z-40 px-4 md:px-8 animate-nav-in">
      <div className="max-w-7xl mx-auto">
        <div className="home-nav-card rounded-[1.75rem] md:rounded-full border border-white/60 flex flex-wrap items-center gap-x-3">
          <button
            type="button"
            onClick={goHome}
            aria-label="回到查词首页"
            className="home-nav-brand flex items-center gap-2.5 shrink-0 whitespace-nowrap"
          >
            <span className="home-nav-logo w-9 h-9 md:w-8 md:h-8 bg-sage rounded-full flex items-center justify-center text-white shadow-sm">
              <BookIcon className="w-4 h-4" />
            </span>
            <span className="font-serif text-lg text-charcoal tracking-wide">
              圈圈<span className="text-clay italic">单词</span>
            </span>
          </button>

          <div className="home-nav-tabs order-3 w-full md:order-none md:w-auto md:ml-6">
            <div className="nav-seg" role="tablist" aria-label="主页分区" onKeyDown={onKeyDown}>
              <span
                aria-hidden="true"
                className="nav-seg-pill"
                style={{ transform: `translate3d(${activeIndex * 100}%, 0, 0)` }}
              />
              {TABS.map((item) => {
                const active = tab === item.id
                return (
                  <button
                    key={item.id}
                    type="button"
                    role="tab"
                    aria-selected={active}
                    aria-current={active ? 'page' : undefined}
                    onClick={() => onTab(item.id)}
                    className={`nav-seg-btn ${active ? 'is-active' : ''}`}
                  >
                    {item.label}
                  </button>
                )
              })}
            </div>
          </div>

          <button
            onClick={onCreate}
            className="home-nav-cta ml-auto inline-flex items-center gap-2 bg-charcoal text-ivory px-4 md:px-5 rounded-full text-sm font-medium hover:bg-charcoal/90 shadow-lg shadow-charcoal/10 whitespace-nowrap shrink-0"
          >
            <PlusIcon className="w-4 h-4" />
            新建单词书
          </button>
        </div>
      </div>
    </nav>
  )
}