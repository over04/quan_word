import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import type { DictEntry, DictProviderId, DictProviderResp } from '../api'
import type { DictApplyMode, DictFillResult } from '../hooks/useDictFill'
import { getProviders } from '../dictCache'
import { useDictLookup } from '../hooks/useDictLookup'
import { useDictSuggest } from '../hooks/useDictSuggest'
import { usePresence } from '../hooks/usePresence'
import {
  ArrowLeftIcon,
  CheckIcon,
  ChevronRightIcon,
  CloseIcon,
  GlobeIcon,
  PlusIcon,
  RefreshIcon,
  SearchIcon,
} from './Icons'
import { toast } from '../toast'

interface Props {
  /** 打开时预填的单词（通常是表单当前拼写） */
  initialWord: string
  /** 面板是否可见：常驻挂载，可见时若输入框还是空的就用表单里的拼写补上 */
  visible: boolean
  /** 右栏形态：常驻在表单右侧，没有遮罩与关闭语义，只有收起 */
  rail?: boolean
  /** 应用词典结果到表单（单条加入不离开面板），返回真实写入/跳过条数 */
  onApply: (entry: DictEntry, mode: DictApplyMode) => DictFillResult
  /** 完成 / 返回表单 */
  onBack: () => void
}

/** 来源矩阵：8 个源的固定顺序与短名（下拉里显示完整名，短名用于窄位兜底） */
const PROVIDER_MATRIX: Array<{ id: DictProviderId; short: string }> = [
  { id: 'youdao', short: '有道' },
  { id: 'baidu', short: '百度' },
  { id: 'eudic', short: '欧路' },
  { id: 'bing', short: '必应' },
  { id: 'iciba', short: '金山' },
  { id: 'free_dict', short: 'Free' },
  { id: 'wiktionary', short: '维基' },
  { id: 'urban', short: 'Urban' },
]

/** 行内「已加入」状态按 来源 + 词 + 内容 记，换源/重查后仍能认出来（不按下标记） */
function defKey(entry: DictEntry, definition: DictEntry['definitions'][number]): string {
  return 'd|' + entry.provider + '|' + entry.word + '|' + definition.pos + '|' + definition.meaning
}

function exampleKey(entry: DictEntry, example: DictEntry['examples'][number]): string {
  return 'e|' + entry.provider + '|' + entry.word + '|' + example.text
}

function phoneticKey(entry: DictEntry): string {
  return 'p|' + entry.provider + '|' + entry.word + '|' + (entry.phonetic ?? '')
}

/** 已加入标记只用于回显，超量后丢最旧的（长会话里不无限增长） */
const ADDED_LIMIT = 240

/* —— 视觉原子：数值全部来自 docs/dev/style.md 与 WordTable / Bookshelf / HomeNav —— */

/** 圆形 44 图标按钮（无描边）：刷新 / 收起 / 返回共用一套悬停反馈（sand/60 圆底） */
const ICON_BTN =
  'inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-charcoal/40 transition-all duration-200 enabled:hover:bg-sand/60 enabled:hover:text-charcoal disabled:cursor-not-allowed disabled:opacity-40'

/** 圆形 44 描边按钮：行尾「+」与「加入音标」共用（1px charcoal/15 描边，hover 描边加深 + 一层 sand） */
const ADD_BTN =
  'inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full border border-charcoal/15 text-charcoal/45 transition-all duration-200 hover:border-charcoal/40 hover:bg-sand/20 hover:text-charcoal'

/** 胶囊次按钮（搜索 / 重试 / 展开例句）：charcoal/20 描边，悬停描边加深 + 一层 sand */
const SECONDARY_BTN =
  'inline-flex min-h-11 shrink-0 items-center justify-center gap-1.5 rounded-full border border-charcoal/20 px-4 text-sm font-medium text-charcoal transition-all duration-200 enabled:hover:border-charcoal/40 enabled:hover:bg-sand/20 disabled:cursor-not-allowed disabled:opacity-40'

/** 徽章：sand/30 底 + sand 描边 + charcoal/70 字（style.md「徽章」） */
const BADGE =
  'inline-flex shrink-0 items-center gap-1.5 rounded-full border border-sand bg-sand/30 px-2.5 py-1 text-xs font-bold tracking-wide text-charcoal/70'

/** 分区小标题：WordTable 表头原样，上方留 16（pt-4） */
const SECTION_TITLE = 'px-4 pb-1.5 pt-4 text-[11px] font-bold tracking-[0.02em] text-charcoal/45'

/** 四态：与结果流同高、垂直居中（状态切换不塌陷） */
const STATE = 'flex min-h-[200px] flex-1 flex-col justify-center px-4 py-6'

/**
 * 网络词典右栏：与表单并置的第二版面（不是面板、不是抽屉），四段——
 * 搜索 56 / 来源下拉 44 / 结果流 1fr / 底栏 56。
 *
 * 视觉按 docs/dev/style.md（Visual Organic）：一张白纸卡（bg-white rounded-3xl
 * shadow-sm border-charcoal/5）、来源是一个「来源 · 当前源 · ⌄」次按钮 + 下拉面板、
 * 结果行靠 divide-y 分隔（行内 padding-block 12 / 正文 15px·1.6），主操作只有底栏「全部加入」。
 *
 * 选源 → 行尾「+」单条加入（不离开面板）→ 底栏「全部加入」补缺后回表单。
 * 已加入的行留在原位转「已加入」徽章（180ms 淡入），换源/重查后仍认得出来。
 * 来源下拉打开时 Esc 只收下拉（preventDefault + stopPropagation，焦点回触发器），
 * 不会冒到 Modal 的 window keydown；下拉关掉后再按 Esc 才由 WordFormModal 收整条词典栏。
 *
 * <768（390 上下分屏）：头部收口成「词头 22 + 音标行（含 44 加入音标）」两行 ≈95，
 * 义项行 68 → 56，规则全在 rail.css 的 max-width:767px 块里（桌面 28 三行 / 68 不变）。
 */
export default function DictSearchPanel({
  initialWord,
  visible,
  rail = false,
  onApply,
  onBack,
}: Props) {
  const [input, setInput] = useState(initialWord)
  const [query, setQuery] = useState('')
  const [entry, setEntry] = useState<DictEntry | null>(null)
  const [picked, setPicked] = useState<number[]>([])
  /** 已加入的行（key 见 defKey/exampleKey/phoneticKey） */
  const [addedKeys, setAddedKeys] = useState<string[]>([])

  /** 一次性行动效：pulse.key 指向刚勾选/刚加入的那一行，token 让勾选框重播缩放 */
  const [pulse, setPulse] = useState<{ key: string; token: number } | null>(null)
  const pulseTimer = useRef<number | null>(null)
  /** 例句默认只摊开 1 条，其余折叠成一行按钮 */
  const [allExamples, setAllExamples] = useState(false)

  /* —— 实时拼写建议：输入即查（400ms 防抖，见 useDictSuggest），下拉里直接点选 ——
     以前只有回车/「搜索」才发请求，拼错一半就什么都看不到，用户没法「看着列表改拼写」。 */
  const [suggestOpen, setSuggestOpen] = useState(false)
  const [suggestActive, setSuggestActive] = useState(0)
  const suggestWrapRef = useRef<HTMLDivElement | null>(null)
  /** 只有用户真的改过输入框，聚焦时才自动摊开建议（打开面板时别一上来就弹列表） */
  const suggestDirtyRef = useRef(false)
  const suggest = useDictSuggest(input)
  const suggestListId = useId()
  /** 后端在没有命中时会回一条「就是你输入的这个」，在面板里它等于噪音：只保留真正的候选 */
  const suggestRows = useMemo(() => {
    const typed = input.trim()
    const rows = suggest.suggestions
    if (rows.length === 1 && rows[0].word === typed) return []
    return rows
  }, [suggest.suggestions, input])
  /** 下拉的退场：关闭后多留 160ms 播淡出（条件渲染本身是瞬间卸载，没有动画） */
  const suggestPop = usePresence(suggestOpen && suggestRows.length > 0, 160)
  /** 退场期间渲染「上一帧的候选」：选中候选后 suggestRows 会立刻清空，
      直接渲染实时值会让退场动画作用在一个 0 高度的空面板上 */
  const suggestRowsRef = useRef(suggestRows)
  useEffect(() => {
    if (suggestRows.length > 0) suggestRowsRef.current = suggestRows
  }, [suggestRows])
  const suggestRowsShown = suggestPop.closing ? suggestRowsRef.current : suggestRows

  /** 结果流：滚到底时摘掉底部遮挡条（data-at-end 直接写 DOM，不触发重渲染） */
  const flowRef = useRef<HTMLDivElement | null>(null)
  /** 白卡根：来源下拉按它的下沿算可用高度（.rail-root 是 overflow-hidden） */
  const rootRef = useRef<HTMLDivElement | null>(null)

  // 词典源列表（会话级单例，失败按「无可用源」处理，不阻塞面板）
  const [providers, setProviders] = useState<DictProviderResp[] | null>(null)
  // 当前查看的来源；结果变化后若失效则回退第一条
  const [source, setSource] = useState<DictProviderId | null>(null)

  /* —— 来源下拉：roving tabindex（当前项 tabIndex 0，其余 -1） —— */
  const [sourceOpen, setSourceOpen] = useState(false)
  const [sourceActive, setSourceActive] = useState(0)
  /** 面板可用高度：不超出白卡下沿与视口，超出时面板内部滚动 */
  const [menuMax, setMenuMax] = useState<number | null>(null)
  const sourceWrapRef = useRef<HTMLDivElement | null>(null)
  /** 面板外层（absolute 定位、不带入场位移）：量可用高度用它，入场动画的 translateY 不干扰测量 */
  const sourceMenuRef = useRef<HTMLDivElement | null>(null)
  const sourceBtnRef = useRef<HTMLButtonElement | null>(null)
  const optionRefs = useRef<Array<HTMLButtonElement | null>>([])
  const sourceListId = useId()

  const handleEntry = useCallback((next: DictEntry | null) => {
    setEntry(next)
    // 换源/重新查询后默认全选义项，例句回到「只摊开 1 条」
    setPicked(next ? next.definitions.map((_, i) => i) : [])
    setAllExamples(false)
  }, [])

  /** 勾选/加入一条时给那一行放一次柔和脉冲（关键帧见 rail.css：railPickSoft / railPickCheck） */
  const firePulse = useCallback((key: string) => {
    setPulse((prev) => ({ key, token: (prev?.token ?? 0) + 1 }))
    if (pulseTimer.current) window.clearTimeout(pulseTimer.current)
    pulseTimer.current = window.setTimeout(() => setPulse(null), 260)
  }, [])

  useEffect(
    () => () => {
      if (pulseTimer.current) window.clearTimeout(pulseTimer.current)
    },
    [],
  )

  // 面板常驻（回到表单不卸载），所以打开时只在输入框为空时补一次当前拼写，
  // 已经查过的词/结果/勾选都保留 —— 回去接着加不用重新搜。
  useEffect(() => {
    if (!visible) return
    setInput((current) => (current.trim() === '' ? initialWord : current))
  }, [visible, initialWord])

  useEffect(() => {
    let alive = true
    getProviders()
      .then((list) => {
        if (alive) setProviders(list)
      })
      .catch(() => {
        if (alive) setProviders([])
      })
    return () => {
      alive = false
    }
  }, [])

  const enabledIds = useMemo(
    () => (providers ?? []).filter((p) => p.enabled).map((p) => p.id),
    [providers],
  )
  const { entries, loading, error, settled, refresh } = useDictLookup(
    query,
    enabledIds,
    enabledIds.length > 0,
  )

  const current = entries.some((item) => item.provider === source) ? source : entries[0]?.provider ?? null
  const result = entries.find((item) => item.provider === current) ?? null

  useEffect(() => {
    handleEntry(result)
  }, [result, handleEntry])

  /** 下拉行：8 个源固定顺序；完整名优先取结果/源列表，没结果的源照样列出来（灰掉） */
  const sourceRows = useMemo(
    () =>
      PROVIDER_MATRIX.map(({ id, short }) => {
        const hit = entries.find((item) => item.provider === id)
        return {
          id,
          short,
          label: hit?.label ?? providers?.find((p) => p.id === id)?.label ?? short,
          available: Boolean(hit),
        }
      }),
    [entries, providers],
  )
  const currentLabel = sourceRows.find((row) => row.id === current)?.label ?? null
  /** 真的能展开（有结果可切换）才算展开：没结果时面板不渲染，aria-expanded 也不该是 true */
  const menuOpen = sourceOpen && Boolean(result)
  /** 来源下拉的退场：关掉后多留 180ms 淡出下沉 */
  const sourceMenu = usePresence(menuOpen, 180)

  // 打开下拉：焦点进当前项；关掉（Esc / 选中 / 点外面）后焦点回按钮
  useEffect(() => {
    if (!sourceOpen) return
    optionRefs.current[sourceActive]?.focus()
    // sourceActive 只在打开这一刻取初值，之后由键盘移动时手动 focus
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sourceOpen])

  // 点面板外收起
  useEffect(() => {
    if (!sourceOpen) return
    const onDown = (e: PointerEvent) => {
      if (!sourceWrapRef.current?.contains(e.target as Node)) setSourceOpen(false)
    }
    document.addEventListener('pointerdown', onDown)
    return () => document.removeEventListener('pointerdown', onDown)
  }, [sourceOpen])

  // 没有可切换的结果（未查询 / 查询中 / 全失败）时不留悬空面板
  useEffect(() => {
    if (!result) setSourceOpen(false)
  }, [result])

  // 面板高度：白卡是 overflow-hidden，超出下沿会被裁掉，所以按「面板顶 → 白卡下沿」算可用高度
  useLayoutEffect(() => {
    if (!sourceOpen) {
      setMenuMax(null)
      return
    }
    const menu = sourceMenuRef.current
    const root = rootRef.current
    if (!menu || !root) return
    const measure = () => {
      const top = menu.getBoundingClientRect().top
      const bottom = Math.min(root.getBoundingClientRect().bottom, window.innerHeight)
      // 至少留 2 行（88），余下内部滚动；再小也放不下就直接贴边滚动
      setMenuMax(Math.max(88, Math.floor(bottom - top - 8)))
    }
    measure()
    window.addEventListener('resize', measure)
    return () => window.removeEventListener('resize', measure)
  }, [sourceOpen])

  function search() {
    const word = input.trim()
    if (word.length < 2) return
    setQuery(word)
    setSuggestOpen(false)
  }

  /** 点建议 = 立刻查这个词（不用再按「搜索」） */
  function pickSuggestion(word: string) {
    setInput(word)
    setQuery(word)
    setSuggestOpen(false)
  }

  /** 输入框键盘：↑↓ 在建议里走、Enter 选当前建议、Esc 只收建议列表 */
  function onSearchKey(e: ReactKeyboardEvent<HTMLInputElement>) {
    const open = suggestOpen && suggestRows.length > 0
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!open) return
      e.preventDefault()
      const next = e.key === 'ArrowDown' ? suggestActive + 1 : suggestActive - 1
      setSuggestActive(Math.min(Math.max(next, 0), suggestRows.length - 1))
      return
    }
    if (e.key === 'Enter' && open) {
      e.preventDefault()
      const row = suggestRows[suggestActive]
      if (row) pickSuggestion(row.word)
      return
    }
    if (e.key === 'Escape' && open) {
      // 只收列表：stopPropagation 否则会冒到 Modal 的 window keydown，整条词典栏一起被收起
      e.preventDefault()
      e.stopPropagation()
      setSuggestOpen(false)
    }
  }

  // 点输入框外面收起建议
  useEffect(() => {
    if (!suggestOpen) return
    const onDown = (e: PointerEvent) => {
      if (!suggestWrapRef.current?.contains(e.target as Node)) setSuggestOpen(false)
    }
    document.addEventListener('pointerdown', onDown)
    return () => document.removeEventListener('pointerdown', onDown)
  }, [suggestOpen])

  function openSourceMenu() {
    const at = Math.max(
      0,
      sourceRows.findIndex((row) => row.id === current),
    )
    setSourceActive(at)
    setSourceOpen(true)
  }

  function closeSourceMenu(restoreFocus = true) {
    setSourceOpen(false)
    if (restoreFocus) sourceBtnRef.current?.focus()
  }

  /**
   * 来源区 Esc（下拉打开时）：只收下拉 —— preventDefault + stopPropagation。
   * Modal 的 Esc 挂在 window 的 keydown 上，事件一旦冒上去会连带收起整条词典栏；
   * 这里拦住后焦点回触发器，再按一次（下拉已关，这里直接放行）才由 WordFormModal 收面板。
   */
  function onSourceAreaKey(e: ReactKeyboardEvent<HTMLDivElement>) {
    if (e.key !== 'Escape' || !menuOpen) return
    e.preventDefault()
    e.stopPropagation()
    closeSourceMenu()
  }

  function moveSourceActive(next: number) {
    const clamped = Math.min(Math.max(next, 0), sourceRows.length - 1)
    setSourceActive(clamped)
    optionRefs.current[clamped]?.focus()
  }

  function chooseSource(id: DictProviderId) {
    setSource(id)
    closeSourceMenu()
  }

  /** 触发键：Enter/Space 由原生 click 打开；↑↓ 也直接打开并落在当前项。
      Esc 不在这里处理 —— 由来源区容器统一拦（见 onSourceAreaKey） */
  function onSourceBtnKey(e: ReactKeyboardEvent<HTMLButtonElement>) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
    e.preventDefault()
    if (menuOpen) moveSourceActive(sourceActive + (e.key === 'ArrowDown' ? 1 : -1))
    else openSourceMenu()
  }

  /** 列表键位：↑↓/Home/End 移动，Enter/Space 由 option 的 click 选中；
      Esc 由来源区容器统一处理（只收下拉，不冒泡到弹窗） */
  function onSourceListKey(e: ReactKeyboardEvent<HTMLDivElement>) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        moveSourceActive(sourceActive + 1)
        break
      case 'ArrowUp':
        e.preventDefault()
        moveSourceActive(sourceActive - 1)
        break
      case 'Home':
        e.preventDefault()
        moveSourceActive(0)
        break
      case 'End':
        e.preventDefault()
        moveSourceActive(sourceRows.length - 1)
        break
      case 'Tab':
        // 先把焦点还给按钮，默认的 Tab 再从按钮往后走一格（不吞掉用户的 Tab）
        closeSourceMenu()
        break
      default:
        break
    }
  }

  function toggleDef(index: number) {
    setPicked((prev) =>
      prev.includes(index) ? prev.filter((x) => x !== index) : [...prev, index].sort((a, b) => a - b),
    )
    firePulse('d' + index)
  }

  /** 行内状态：加入成功后该行转「已加入」（重复跳过也算已在表单里） */
  function markAdded(keys: string[]) {
    if (keys.length === 0) return
    setAddedKeys((prev) => {
      const next = [...prev, ...keys.filter((key) => !prev.includes(key))]
      return next.length > ADDED_LIMIT ? next.slice(next.length - ADDED_LIMIT) : next
    })
  }

  /** 单条加入：不离开面板，可以接着勾、接着加 */
  function addDefinition(definition: DictEntry['definitions'][number], index: number) {
    if (!entry) return
    const fill = onApply({ ...entry, definitions: [definition] }, 'definitions')
    if (fill.added > 0 || fill.skipped > 0) markAdded([defKey(entry, definition)])
    firePulse('d' + index)
    toast(fill.added > 0 ? '已加入 1 条释义' : '这条已经在表单里了')
  }

  function addExample(example: DictEntry['examples'][number], index: number) {
    if (!entry) return
    const fill = onApply({ ...entry, examples: [example] }, 'example')
    if (fill.added > 0 || fill.skipped > 0) markAdded([exampleKey(entry, example)])
    firePulse('e' + index)
    toast(fill.added > 0 ? '已加入例句' : '这句已经在表单里了')
  }

  function addPhonetic() {
    if (!entry?.phonetic) return
    const fill = onApply(entry, 'phonetic')
    if (fill.added > 0 || fill.skipped > 0) markAdded([phoneticKey(entry)])
    toast(fill.added > 0 ? '已加入音标' : '音标已经在表单里了')
  }

  /** 全部加入：只补缺，不覆盖任何已有内容 */
  function addAll() {
    if (!entry) return
    const defs = entry.definitions.filter((_, i) => picked.includes(i))
    let added = 0
    if (defs.length > 0) added += onApply({ ...entry, definitions: defs }, 'definitions').added
    added += onApply(entry, 'phonetic').added
    if (entry.examples.length > 0) {
      added += onApply({ ...entry, examples: entry.examples.slice(0, 1) }, 'example').added
    }
    // 面板关掉后再打开，行内状态要跟得上：本次一起补进去的都转「已加入」
    markAdded([
      ...defs.map((definition) => defKey(entry, definition)),
      ...(entry.phonetic ? [phoneticKey(entry)] : []),
      ...(entry.examples.length > 0 ? [exampleKey(entry, entry.examples[0])] : []),
    ])
    toast(added > 0 ? '已加入 ' + added + ' 项' : '没有新内容')
    onBack()
  }

  /** 单条例句行（第一条常显，其余在 .collapse 里）。entry 显式传入，不用 `result as DictEntry` 断言 */
  function renderExampleRow(
    entry: DictEntry,
    example: DictEntry['examples'][number],
    index: number,
  ) {
    const added = addedKeys.includes(exampleKey(entry, example))
    const pulseToken = pulse && pulse.key === 'e' + index ? pulse.token : null
    return (
      <li
        key={index}
        className="animate-result-in"
        style={{ animationDelay: Math.min(index, 7) * 28 + 'ms' }}
      >
        <div
          className="flex items-center gap-2 px-4 py-3 transition-colors duration-200 hover:bg-sand/20"
          data-pulse={pulseToken ? 'true' : undefined}
        >
          <div className="min-w-0 flex-1">
            <p className="break-words text-[15px] font-medium leading-[1.6] text-charcoal/70">
              {example.text}
            </p>
            {example.translation && (
              <p className="mt-1 break-words text-[13px] leading-[1.6] text-charcoal/40">
                {example.translation}
              </p>
            )}
          </div>
          {added ? (
            <span className={BADGE + ' rail-added'}>已加入</span>
          ) : (
            <button
              type="button"
              onClick={() => addExample(example, index)}
              aria-label={'加入第 ' + (index + 1) + ' 条例句'}
              title="加入这句例句"
              className={ADD_BTN}
            >
              <PlusIcon className="h-4 w-4" />
            </button>
          )}
        </div>
      </li>
    )
  }

  const hasPhonetic = Boolean(entry?.phonetic)
  const hasExamples = Boolean(entry && entry.examples.length > 0)
  /** 只有真发起了查询才谈得上「查询中」：空 query 直接进空态（「未查询」大字），
      否则 settled 永远为 false，面板会一直停在骨架上 */
  const hasQuery = query.trim().length >= 2
  /** 源列表未到 / 查询中 / 尚未落定：都显示骨架，不留空白也不闪「没有结果」 */
  const pending =
    !error && hasQuery && (providers === null || loading || (enabledIds.length > 0 && !settled))
  /** 义项多选行（只有一条义项时不需要勾选，与旧行为一致） */
  const selectableDefs = Boolean(result && result.definitions.length > 1)
  /** 例句：默认 1 条，其余折叠（折叠部分用 .collapse 平滑展开） */
  const hiddenExamples = result ? Math.max(0, result.examples.length - 1) : 0
  const sourceText = currentLabel ?? (pending ? '查询中' : '未查询')

  return (
    <div
      ref={rootRef}
      className="rail-root relative flex min-h-0 w-full min-w-0 flex-1 animate-fade-in-up flex-col overflow-hidden rounded-2xl border border-charcoal/5 bg-white text-charcoal shadow-sm"
    >
      {/* 右栏形态没有标题条：父级工具条已有「词典」开关；保留 h2 语义供对话框标注 */}
      {rail ? (
        <h2 className="sr-only">网络词典</h2>
      ) : (
        <div className="flex min-h-14 items-center gap-2 border-b border-charcoal/5 px-3">
          <button type="button" onClick={onBack} aria-label="返回表单" className={ICON_BTN}>
            <ArrowLeftIcon className="h-4 w-4" />
          </button>
          <h2 className="min-w-0 truncate font-serif text-lg text-charcoal">网络词典</h2>
          <button
            type="button"
            onClick={onBack}
            aria-label="返回表单"
            className={ICON_BTN + ' ml-auto'}
          >
            <CloseIcon className="h-4 w-4" />
          </button>
        </div>
      )}

      {/* ① 搜索 56：输入框 + 胶囊次按钮 + 图标按钮（焦点环由全局负责，这里只给 clay 描边） */}
      <form
        className="flex min-h-14 flex-none items-center gap-1.5 border-b border-charcoal/5 px-3"
        onSubmit={(e) => {
          e.preventDefault()
          search()
        }}
      >
        <div ref={suggestWrapRef} className="relative min-w-0 flex-1">
          <SearchIcon className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-charcoal/35" />
          <input
            value={input}
            onChange={(e) => {
              setInput(e.target.value)
              suggestDirtyRef.current = true
              setSuggestActive(0)
              setSuggestOpen(true)
            }}
            onFocus={() => {
              if (suggestDirtyRef.current) setSuggestOpen(true)
            }}
            onKeyDown={onSearchKey}
            role="combobox"
            aria-autocomplete="list"
            aria-expanded={suggestOpen && suggestRows.length > 0}
            aria-controls={suggestListId}
            aria-activedescendant={
              suggestOpen && suggestRows.length > 0 ? suggestListId + '-opt-' + suggestActive : undefined
            }
            aria-label="要查询的单词"
            placeholder="abandon"
            autoComplete="off"
            style={{ WebkitAppearance: 'none', appearance: 'none' }}
            autoFocus
            className="min-h-11 w-full rounded-xl border border-charcoal/15 bg-white pl-9 pr-3 text-sm text-charcoal placeholder:text-charcoal/30 transition-colors focus:border-clay focus:outline-none"
          />

          {/* 拼写建议：输入 ≥2 字符即出（防抖 400ms）。点一行 = 直接查这个词，
              这样「半对半错」的拼写也能从候选里挑，不用先背对再查 */}
          {suggestPop.mounted && (
            <div
              data-state={suggestOpen && suggestRows.length > 0 ? 'open' : 'closing'}
              className="ui-pop absolute left-0 right-0 top-full z-30 mt-1 overflow-hidden rounded-xl border border-charcoal/10 bg-white shadow-lg shadow-charcoal/10"
            >
              <ul
                id={suggestListId}
                role="listbox"
                aria-label="拼写建议"
                className="max-h-[264px] overflow-y-auto"
              >
                {suggestRowsShown.map((item, i) => (
                  <li
                    key={item.word + '|' + i}
                    id={suggestListId + '-opt-' + i}
                    role="option"
                    aria-selected={i === suggestActive}
                    onMouseDown={(e) => e.preventDefault()}
                    onMouseEnter={() => setSuggestActive(i)}
                    onClick={() => pickSuggestion(item.word)}
                    className={
                      'flex min-h-11 cursor-pointer items-center gap-2 px-3 transition-colors ' +
                      (i === suggestActive ? 'bg-sand/40' : 'hover:bg-sand/20')
                    }
                  >
                    <span className="shrink-0 font-serif text-[15px] leading-tight text-charcoal">
                      {item.word}
                    </span>
                    {item.summary && (
                      <span className="min-w-0 flex-1 truncate text-xs text-charcoal/45">
                        {item.summary}
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
        <button type="submit" disabled={input.trim().length < 2} className={SECONDARY_BTN + ' px-3.5'}>
          搜索
        </button>
        <button
          type="button"
          onClick={refresh}
          aria-label="刷新词典"
          title="刷新词典"
          disabled={!query}
          className={ICON_BTN}
        >
          <RefreshIcon className="h-4 w-4" />
        </button>
        {rail && (
          <button type="button" onClick={onBack} aria-label="收起词典" className={ICON_BTN}>
            <CloseIcon className="h-4 w-4" />
          </button>
        )}
      </form>

      {/* ② 来源：一行「来源 · 当前源 · ⌄」次按钮（44），点开才是 8 行下拉 */}
      <div
        ref={sourceWrapRef}
        onKeyDown={onSourceAreaKey}
        className="relative flex-none border-b border-charcoal/5 px-3 py-2"
      >
        <button
          ref={sourceBtnRef}
          type="button"
          aria-haspopup="listbox"
          aria-expanded={menuOpen}
          aria-controls={menuOpen ? sourceListId : undefined}
          disabled={!result}
          onClick={() => (menuOpen ? closeSourceMenu() : openSourceMenu())}
          onKeyDown={onSourceBtnKey}
          className="flex h-11 w-full items-center gap-2 rounded-full border border-charcoal/20 px-4 text-left text-sm font-medium text-charcoal transition-all duration-200 enabled:hover:border-charcoal/40 enabled:hover:bg-sand/20 disabled:cursor-not-allowed disabled:opacity-40"
        >
          <span className="shrink-0 text-xs font-bold uppercase tracking-widest text-charcoal/40">
            来源
          </span>
          <span
            className={
              'min-w-0 truncate ' + (result ? 'text-charcoal' : 'font-normal text-charcoal/40')
            }
          >
            {sourceText}
          </span>
          <ChevronRightIcon
            className={
              'ml-auto h-4 w-4 shrink-0 text-charcoal/40 transition-transform duration-200 ' +
              (menuOpen ? '-rotate-90' : 'rotate-90')
            }
          />
        </button>

        {sourceMenu.mounted && (
          <div
            ref={sourceMenuRef}
            data-state={menuOpen ? 'open' : 'closing'}
            className="ui-pop absolute left-3 right-3 top-full z-20"
          >
            <div
              id={sourceListId}
              role="listbox"
              aria-label="词典来源"
              onKeyDown={onSourceListKey}
              className="rail-menu rounded-xl border border-charcoal/10 bg-white px-2 py-1.5 shadow-lg"
              style={menuMax ? { maxHeight: menuMax } : undefined}
            >
              {sourceRows.map((row, index) => {
                const selected = row.id === current
                return (
                  <button
                    key={row.id}
                    ref={(el) => {
                      optionRefs.current[index] = el
                    }}
                    type="button"
                    role="option"
                    aria-selected={selected}
                    aria-disabled={row.available ? undefined : true}
                    tabIndex={index === sourceActive ? 0 : -1}
                    onClick={() => {
                      if (row.available) chooseSource(row.id)
                    }}
                    onFocus={() => setSourceActive(index)}
                    className={
                      'flex min-h-11 w-full items-center gap-2 rounded-lg px-2.5 text-left text-sm transition-colors duration-150 ' +
                      (selected
                        ? 'font-medium text-charcoal'
                        : row.available
                          ? 'text-charcoal/70 hover:bg-sand/20 hover:text-charcoal'
                          : 'text-charcoal/35')
                    }
                  >
                    <span className="flex h-4 w-4 shrink-0 items-center justify-center">
                      {selected && <CheckIcon className="h-4 w-4 text-clay" />}
                    </span>
                    <span className="min-w-0 flex-1 truncate">{row.label}</span>
                    {!row.available && (
                      <span className="shrink-0 text-[11px] text-charcoal/30">无结果</span>
                    )}
                  </button>
                )
              })}
            </div>
          </div>
        )}
      </div>

      {/* ③ 结果流：空 / 加载 / 无结果 / 出错 四态（骨架与空态都占 min-h-200） */}
      <div ref={flowRef} className="rail-flow">
        {pending ? (
          <div className={STATE + ' animate-pulse'} aria-hidden="true">
            <div className="h-8 w-32 rounded-xl bg-sand/30" />
            <div className="mt-2 h-4 w-24 rounded-xl bg-sand/30" />
            <div className="mt-6 space-y-3">
              <div className="h-12 rounded-xl bg-sand/30" />
              <div className="h-12 rounded-xl bg-sand/30" />
              <div className="h-12 rounded-xl bg-sand/30" />
            </div>
          </div>
        ) : error ? (
          <div className={STATE} role="status">
            <p className="font-serif text-2xl leading-none text-charcoal">离线</p>
            <p className="mt-2 text-xs leading-relaxed text-charcoal/40">{error}</p>
            <button type="button" onClick={refresh} className={SECONDARY_BTN + ' mt-4 self-start'}>
              <RefreshIcon className="h-3.5 w-3.5" />
              重试
            </button>
          </div>
        ) : result ? (
          <div key={result.provider + '|' + query} className="animate-fade-in-up">
            {/* 词头：衬线大字 28 + 音标 + 44 加入音标；来源徽章退到下一行小字。
                <768 由 rail.css 收口（.rail-head/.rail-word/.rail-phon/.rail-source）：
                词头 22、音标与「加入音标」同行内联、来源徽章行隐藏（来源看上方下拉），
                头部 143 → ≈95，分屏下才留得出义项的位置。 */}
            <div className="rail-head border-b border-charcoal/5 px-4 pb-3 pt-4">
              <h3 className="rail-word break-words font-serif text-[28px] leading-[1.15] text-charcoal">
                {result.word}
              </h3>
              {result.phonetic && (
                <div className="rail-phon mt-1 flex items-center gap-2">
                  <span className="min-w-0 break-words font-serif text-base leading-snug text-charcoal/70">
                    {result.phonetic}
                  </span>
                  {addedKeys.includes(phoneticKey(result)) ? (
                    <span className={BADGE + ' rail-added'}>已加入</span>
                  ) : (
                    <button
                      type="button"
                      onClick={addPhonetic}
                      aria-label="加入音标"
                      title="加入音标"
                      className={ADD_BTN + ' rail-add'}
                    >
                      <PlusIcon className="h-4 w-4" />
                    </button>
                  )}
                </div>
              )}
              <div className="rail-source mt-2 flex flex-wrap items-center gap-x-3">
                {result.source_url && (
                  <a
                    href={result.source_url}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-flex min-h-11 min-w-11 items-center gap-1 text-xs font-medium text-charcoal/40 transition-colors hover:text-charcoal"
                  >
                    <GlobeIcon className="h-3.5 w-3.5" />
                    原文
                  </a>
                )}
              </div>
            </div>

            {result.definitions.length > 0 && (
              <>
                <p className={SECTION_TITLE}>释义</p>
                <ul className="divide-y divide-charcoal/5">
                  {result.definitions.map((definition, index) => {
                    const added = addedKeys.includes(defKey(result, definition))
                    const checked = picked.includes(index)
                    // pickPulse 的 token：变化时让勾选框重挂一次，缩放动画重播
                    const pulseToken = pulse && pulse.key === 'd' + index ? pulse.token : null
                    const body = (
                      <span
                        className={
                          'rail-def-text min-w-0 break-words text-[15px] font-medium leading-[1.6] ' +
                          (added ? 'text-charcoal/60' : 'text-charcoal')
                        }
                      >
                        {definition.pos && (
                          <span className="mr-2 align-baseline text-[11px] font-bold tracking-wide text-charcoal/40">
                            {definition.pos}
                          </span>
                        )}
                        {definition.meaning}
                      </span>
                    )
                    return (
                      <li
                        key={index}
                        className="animate-result-in"
                        style={{ animationDelay: Math.min(index, 7) * 28 + 'ms' }}
                      >
                        <div
                          className="rail-def-row flex items-start gap-2 px-4 py-3 transition-colors duration-200 hover:bg-sand/20"
                          data-pulse={pulseToken ? 'true' : undefined}
                          data-picked={selectableDefs && checked ? 'true' : undefined}
                        >
                          {selectableDefs ? (
                            // 整行就是命中区（≥44），勾选框只是状态指示
                            <button
                              type="button"
                              onClick={() => toggleDef(index)}
                              aria-pressed={checked}
                              aria-label={'选择第 ' + (index + 1) + ' 条释义'}
                              className="rail-def-body flex min-h-11 min-w-0 flex-1 items-start gap-2.5 py-2.5 text-left"
                            >
                              <span
                                key={pulseToken ?? 'idle'}
                                className="rail-check"
                                aria-hidden="true"
                              >
                                <CheckIcon className="rail-check-ico" />
                              </span>
                              {body}
                            </button>
                          ) : (
                            <div className="rail-def-body flex min-h-11 min-w-0 flex-1 items-start gap-2.5 py-2.5">
                              {body}
                            </div>
                          )}
                          {added ? (
                            <span className={BADGE + ' rail-added mt-2.5'}>已加入</span>
                          ) : (
                            <button
                              type="button"
                              onClick={() => addDefinition(definition, index)}
                              aria-label={'加入第 ' + (index + 1) + ' 条释义'}
                              title="加入这条释义"
                              className={ADD_BTN + ' rail-add'}
                            >
                              <PlusIcon className="h-4 w-4" />
                            </button>
                          )}
                        </div>
                      </li>
                    )
                  })}
                </ul>
              </>
            )}

            {result.examples.length > 0 && (
              <>
                <p className={SECTION_TITLE}>例句</p>
                <ul className="divide-y divide-charcoal/5">
                  {result.examples
                    .slice(0, 1)
                    .map((example, index) => renderExampleRow(result, example, index))}
                </ul>
                {/* 其余例句：0fr ↔ 1fr 平滑展开/收起（高度未知也能插值，不用测） */}
                <div className="collapse" data-open={allExamples ? 'true' : 'false'}>
                  <div>
                    <ul className="divide-y divide-charcoal/5 border-t border-charcoal/5">
                      {result.examples
                        .slice(1)
                        .map((example, i) => renderExampleRow(result, example, i + 1))}
                    </ul>
                  </div>
                </div>
                {hiddenExamples > 0 && (
                  <div className="px-4 pb-4 pt-1.5">
                    <button
                      type="button"
                      onClick={() => setAllExamples((open) => !open)}
                      aria-expanded={allExamples}
                      className="inline-flex min-h-11 w-full items-center justify-center gap-1.5 rounded-full border border-charcoal/15 px-4 text-xs font-medium text-charcoal/60 transition-all duration-200 hover:border-charcoal/30 hover:bg-sand/20 hover:text-charcoal"
                    >
                      {allExamples ? '收起例句' : '另有 ' + hiddenExamples + ' 条例句'}
                    </button>
                  </div>
                )}
              </>
            )}
          </div>
        ) : (
          <div className={STATE} role={query ? 'status' : undefined}>
            <p className="break-words font-serif text-2xl leading-none text-charcoal">
              {query || 'Aa'}
            </p>
            <p className="mt-2 text-xs text-charcoal/40">{query ? '没有结果' : '未查询'}</p>
          </div>
        )}

      </div>

      {/* ④ 底栏 56：全局唯一实心块 = 主操作；单条加入在行尾的 + 上 */}
      <div className="flex min-h-14 flex-none items-center gap-4 border-t border-charcoal/5 px-4 py-3">
        <span className="shrink-0 whitespace-nowrap text-xs text-charcoal/40">
          已选 {picked.length} 条
        </span>
        <button
          type="button"
          onClick={addAll}
          disabled={!entry || (picked.length === 0 && !hasPhonetic && !hasExamples)}
          className="inline-flex min-h-11 min-w-0 flex-1 items-center justify-center rounded-full bg-charcoal px-5 text-sm font-medium text-ivory shadow-lg shadow-charcoal/10 transition-all duration-200 enabled:hover:bg-charcoal/90 disabled:cursor-not-allowed disabled:bg-charcoal/20 disabled:text-charcoal/40 disabled:shadow-none"
        >
          全部加入
        </button>
      </div>
    </div>
  )
}
