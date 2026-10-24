import { useEffect, useRef, useState } from 'react'
import { words, type Definition, type DictEntry, type Tag, type Word } from '../api'
import { exampleText, useDictFill, type DictApplyMode, type DictFillResult } from '../hooks/useDictFill'
import { useDiscardGuard, useUnsavedChanges } from '../hooks/useUnsavedChanges'
import { useTagDraft } from '../hooks/useTagDraft'
import type { WordDraft } from '../wordDraft'
import DictSearchPanel from './DictSearchPanel'
import { usePresence } from '../hooks/usePresence'
import DiscardConfirmModal from './DiscardConfirmModal'
import Modal from './Modal'
import TagPicker from './TagPicker'
import { ArrowLeftIcon, CheckIcon, ChevronRightIcon, CloseIcon, GlobeIcon, PlusIcon } from './Icons'

/** 词典回填后的持久状态（字段下沿一行：项目徽章 + 「撤销」文字按钮） */
interface FillNote {
  text: string
  /** 动作文案，默认「撤销」；用于「词典拼写与原值不同」时的「改用」 */
  actionLabel?: string
  /** 没有可撤销的动作时（如「没有新增」）不渲染按钮 */
  action?: () => void
}

/** 两位数字（落款时刻） */
function pad2(n: number): string {
  return String(n).padStart(2, '0')
}

/** 保存成功的时间戳（主按钮换成「已存 12:04」） */
function nowStamp(): string {
  const d = new Date()
  return pad2(d.getHours()) + ':' + pad2(d.getMinutes())
}

/**
 * 词典回填状态：被填字段下沿的一行 —— 项目徽章（做了什么）+「撤销」文字按钮（44 命中区）。
 * added === 0 时只写「没有新增」且没有按钮；「已…」的徽章带一枚 clay 圆点（同 style.md 徽章）。
 */
function FillNoteRow({ note, className = '' }: { note: FillNote; className?: string }) {
  const added = note.text.startsWith('已')
  return (
    <p className={'flex flex-wrap items-center gap-2 ' + className}>
      <span className="inline-flex items-center gap-1.5 rounded-full border border-sand bg-sand/30 px-2.5 py-1 text-xs font-bold tracking-wide text-charcoal/70">
        {added && <span aria-hidden="true" className="h-1.5 w-1.5 rounded-full bg-clay" />}
        {note.text}
      </span>
      {note.action && (
        <button
          type="button"
          onClick={note.action}
          className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-full px-2 text-xs font-medium text-clay transition-colors duration-200 hover:bg-clay/10 hover:text-charcoal active:scale-95"
        >
          {note.actionLabel ?? '撤销'}
        </button>
      )}
    </p>
  )
}

interface Props {
  bookId: number
  initial: Word | null
  /** 词典抓取带入的初始草稿（仅新建时生效；initial 优先） */
  initialDraft?: WordDraft | null
  onClose: () => void
  onSaved: () => void
  /** 该书全部标签 */
  tags: Tag[]
  /** 新建标签成功后回调（父级刷新标签列表，仅内存同步） */
  onTagsCreated: (tag: Tag) => void
}

/** 词性枚举：下拉选择，防止手输错误；C 可数 / U 不可数（名词），vt./vi. 及物/不及物（动词） */
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

/** 分区标题：11px 粗体 + 极轻字距（中文没有大小写，不套 uppercase，只保留颜色与重量区分） */
const SECTION_TITLE = 'text-[11px] font-bold tracking-[0.02em] text-charcoal/45'

/** 计数：标题右侧的弱化小字（替代原来的条数翻牌） */
const SECTION_COUNT = 'text-xs tabular-nums text-charcoal/40'

/** 输入框：白底 + charcoal/15 描边 + 圆角，焦点描边转 clay（style.md「输入」） */
const FIELD =
  'min-h-11 rounded-xl border border-charcoal/15 bg-white text-sm text-charcoal transition-colors duration-200 placeholder:text-charcoal/30 focus:border-clay'

/** 圆形图标按钮：44 命中区，hover 只换底色/文字色（TagManageModal / DictSearchPanel 同款） */
const ICON_BTN =
  'inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full transition-all duration-200 active:scale-95'

/**
 * 添加 / 编辑单词：Modal 的 page 形态（bg-ivory + 环境底图）。
 *
 * 版面（1329×800 不滚动即可看到词头 / 音标 / 释义 / 例句 / 标签）：
 *   顶条（返回 44 · 衬线标题 · 未保存 · 词典 · 保存）
 *   两张卡并排、同一个父级 .wb-body（flex 拉伸 → top / bottom 完全对齐，间距 20px）：
 *     └ .wb-sheet 纸卡（#FDFAF4 / rounded-2xl / shadow-sm / border-charcoal/5，下层垫 #F2EAE0 位移 8px）
 *          └ 表单（词头衬线大字 + clay 底线 / 音标 / 释义账本 / 例句 / 标签）
 *     └ .wb-rail 词典白卡（rail.css 的 .rail-root，与纸卡同高）
 *   <768 折成「纸卡 42% / 把手 44 / 词典」的上下分屏：纸卡吃正文高度的 42%，词典白卡吃剩余高度。
 *
 * 视觉全部走项目工具类与 docs/dev/style.md 的词汇表；bench.css 只剩两张卡的外框、滚动条、
 * 结果流底部留白与词典栏容器。
 */
export default function WordFormModal({
  bookId,
  initial,
  initialDraft,
  onClose,
  onSaved,
  tags,
  onTagsCreated,
}: Props) {
  // 初始快照：仅首次渲染取值；用于判断是否有未保存更改
  const snapshot = useRef({
    spelling: initial?.spelling ?? initialDraft?.spelling ?? '',
    phonetic: initial?.phonetic ?? initialDraft?.phonetic ?? '',
    definitions:
      initial?.definitions ??
      initialDraft?.definitions ??
      ([{ pos: '', meaning: '' }] as Definition[]),
    example: initial?.example ?? initialDraft?.example ?? '',
  }).current

  const [spelling, setSpelling] = useState(snapshot.spelling)
  const [phonetic, setPhonetic] = useState(snapshot.phonetic)
  const [definitions, setDefinitions] = useState<Definition[]>(snapshot.definitions)
  const [example, setExample] = useState(snapshot.example)
  const tagDraft = useTagDraft(initial?.tags ?? [], tags, onTagsCreated)
  const [error, setError] = useState('')
  const [saving, setSaving] = useState(false)
  // 宽屏默认把词典栏打开（工作台形态：表单与词典并置）；窄屏保持单栏，按需展开
  const [dictOpen, setDictOpen] = useState(
    () => typeof window !== 'undefined' && window.matchMedia('(min-width: 1024px)').matches,
  )
  /** 收起词典也要有退场：节点多留 260ms，样式侧按 data-state='closing' 播收列宽 / 滑下去 */
  const rail = usePresence(dictOpen, 260)
  // 保存成功的时间戳：有值时主按钮换成「已存 12:04」并淡入一枚 check
  const [sealed, setSealed] = useState<string | null>(null)
  // 词典回填后的持久状态：字段旁显示「已填入/已追加」，可撤销（再次操作也能覆盖）
  const [notes, setNotes] = useState<{
    word?: FillNote
    phonetic?: FillNote
    definitions?: FillNote
    example?: FillNote
  }>({})
  const spellingRef = useRef<HTMLInputElement>(null)
  const phoneticRef = useRef<HTMLInputElement>(null)
  const definitionsRef = useRef<HTMLDivElement>(null)
  const exampleRef = useRef<HTMLTextAreaElement>(null)
  const dictTriggerRef = useRef<HTMLButtonElement>(null)
  // 关闭面板后焦点落点：'trigger'（默认）或回填过的字段
  const focusTargetRef = useRef<'trigger' | 'phonetic' | 'definitions' | 'example'>('trigger')
  const dictFill = useDictFill({
    phonetic,
    definitions,
    example,
    setPhonetic,
    setDefinitions,
    setExample,
  })

  // 未保存 = 改过的字段数（顶条「未保存 N 处」按处计数，语义与原 dirty 布尔一致）
  const changedFields = [
    spelling !== snapshot.spelling,
    phonetic !== snapshot.phonetic,
    JSON.stringify(definitions) !== JSON.stringify(snapshot.definitions),
    example !== snapshot.example,
    tagDraft.dirty,
  ].filter(Boolean).length
  const dirty = changedFields > 0

  const guard = useDiscardGuard(dirty, onClose)
  useUnsavedChanges(dirty)

  function requestClose() {
    if (saving || dictOpen) return
    guard.requestClose()
  }

  /** 关闭/返回：词典面板打开时先回表单（单层交互，Esc 与遮罩同此行为） */
  function modalClose() {
    if (saving) return
    if (dictOpen) {
      closeDict()
      return
    }
    requestClose()
  }

  function closeDict() {
    setDictOpen(false)
    const target = focusTargetRef.current
    focusTargetRef.current = 'trigger'
    // 回到表单后把焦点还给触发按钮（回填过则落到被回填的字段）
    requestAnimationFrame(() => {
      const el =
        target === 'phonetic'
          ? phoneticRef.current
          : target === 'definitions'
            ? definitionsRef.current
            : target === 'example'
              ? exampleRef.current
              : dictTriggerRef.current
      el?.focus({ preventScroll: true })
    })
  }

  /**
   * 词典回填：写回表单 + 字段旁留下可撤销的状态说明。
   * 每个字段的「撤销」只回滚该字段（曾经三个字段共用同一个 restoreAll，
   * 点音标旁的撤销会把释义、例句一起清空）。
   */
  function handleDictApply(entry: DictEntry, mode: DictApplyMode): DictFillResult {
    const prev = { spelling, phonetic, definitions, example }
    const result = dictFill.applyFill(entry, mode)
    focusTargetRef.current =
      mode === 'phonetic' ? 'phonetic' : mode === 'definitions' ? 'definitions' : mode === 'example' ? 'example' : 'trigger'

    const clearNote = (key: 'word' | 'phonetic' | 'definitions' | 'example') =>
      setNotes((n) => ({ ...n, [key]: undefined }))

    // 单词：空表单直接补上（否则保存必报「单词不能为空」）；已有拼写不覆盖，只提供「改用」
    const typed = prev.spelling.trim()
    if (!typed) {
      setSpelling(entry.word)
      setNotes((n) => ({
        ...n,
        word: {
          text: '已填入单词',
          action: () => {
            setSpelling(prev.spelling)
            clearNote('word')
          },
        },
      }))
    } else if (typed.toLowerCase() !== entry.word.toLowerCase()) {
      setNotes((n) => ({
        ...n,
        word: {
          text: '词典为「' + entry.word + '」',
          actionLabel: '改用',
          action: () => {
            setSpelling(entry.word)
            setNotes((n2) => ({
              ...n2,
              word: {
                text: '已填入单词',
                action: () => {
                  setSpelling(prev.spelling)
                  setNotes((n3) => ({ ...n3, word: undefined }))
                },
              },
            }))
          },
        },
      }))
    }

    const exampleFilled = exampleText(entry)
    if (mode === 'overwrite') {
      setNotes((n) => ({
        ...n,
        phonetic: entry.phonetic
          ? {
              text: '已填入音标',
              action: () => {
                setPhonetic(prev.phonetic)
                clearNote('phonetic')
              },
            }
          : n.phonetic,
        definitions:
          entry.definitions.length > 0
            ? {
                text: '已填入 ' + entry.definitions.length + ' 条释义',
                action: () => {
                  setDefinitions(prev.definitions)
                  clearNote('definitions')
                },
              }
            : n.definitions,
        example: exampleFilled
          ? {
              text: '已填入例句',
              action: () => {
                setExample(prev.example)
                clearNote('example')
              },
            }
          : n.example,
      }))
      return result
    }
    if (mode === 'phonetic') {
      setNotes((n) => ({
        ...n,
        phonetic:
          result.added > 0
            ? {
                text: '已填入音标',
                action: () => {
                  setPhonetic(prev.phonetic)
                  clearNote('phonetic')
                },
              }
            : { text: '与表单一致，没有改动' },
      }))
      return result
    }
    if (mode === 'definitions') {
      setNotes((n) => ({
        ...n,
        definitions:
          result.added > 0
            ? {
                text: '已追加 ' + result.added + ' 条释义',
                action: () => {
                  setDefinitions(prev.definitions)
                  clearNote('definitions')
                },
              }
            : { text: '没有新增（这些义项都已存在）' },
      }))
      return result
    }
    setNotes((n) => ({
      ...n,
      example:
        result.added > 0
          ? {
              text: '已追加 1 条例句',
              action: () => {
                setExample(prev.example)
                clearNote('example')
              },
            }
          : { text: '例句已存在，没有新增' },
    }))
    return result
  }

  function updateDef(i: number, patch: Partial<Definition>) {
    setDefinitions((ds) => ds.map((d, idx) => (idx === i ? { ...d, ...patch } : d)))
  }

  /** 加入一条释义：新行直接追加（入场用 animate-word-rise，只有新挂载的那一行播） */
  function addDefinition() {
    setDefinitions((ds) => [...ds, { pos: '', meaning: '' }])
  }

  /** 删除一条释义：立即移除（与列表模式一致，不留划掉/收行的等待期） */
  function removeDefinition(index: number) {
    setDefinitions((ds) => ds.filter((_, idx) => idx !== index))
  }

  async function submit() {
    if (saving) return
    if (!spelling.trim()) {
      setError('单词不能为空')
      return
    }
    if (definitions.length === 0 || definitions.some((d) => !d.meaning.trim())) {
      setError('至少需要一个释义，且释义内容不能为空')
      return
    }
    setSaving(true)
    setError('')
    try {
      // 先创建待创建标签（失败时已创建的标签会保留并转为普通标签，可直接重试）
      const tagIds = await tagDraft.resolve(bookId)
      const payload = {
        spelling: spelling.trim(),
        phonetic: phonetic.trim() || undefined,
        definitions: definitions.map((d) => ({ pos: d.pos.trim(), meaning: d.meaning.trim() })),
        example: example.trim() || undefined,
        tags: tagIds,
      }
      if (initial) {
        await words.update(bookId, initial.id, payload)
      } else {
        await words.create(bookId, payload)
      }
      // 保存成功：主按钮换成「已存 12:04」+ 一枚淡入的 check（轻动效，0.46s 后交给父级关闭）
      setSealed(nowStamp())
      await new Promise<void>((resolve) => {
        window.setTimeout(resolve, 460)
      })
      onSaved()
    } catch (e) {
      setError(e instanceof Error ? e.message : '保存失败')
      setSaving(false)
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    void submit()
  }

  // Ctrl/⌘+S：挂在 window 上，焦点在顶条按钮/页面任意位置都能保存（原来挂在 <form> 上，
  // 顶条的保存/词典/返回都在 form 之外，按了没反应）
  const submitRef = useRef<() => void>(() => {})
  submitRef.current = () => void submit()
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault()
        submitRef.current()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const draftState = dirty && !sealed
  const primaryLabel = sealed ? '已存 ' + sealed : saving ? '保存中' : '保存'

  return (
    <Modal onClose={modalClose} page>
      <div className="flex min-h-0 flex-1 flex-col px-4 pt-4 pb-4 md:px-6 md:pt-4 md:pb-5">
        {/* 顶条：返回 · 标题 · 未保存 · 词典 / 保存。
            <768 折成两行（第一行 44 动作、第二行标题），主操作始终在首屏。 */}
        <header className="mx-auto w-full max-w-[1400px] shrink-0">
          <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2 py-1 md:grid-cols-[auto_minmax(0,1fr)_auto_auto] md:gap-x-3">
            <button
              type="button"
              onClick={modalClose}
              aria-label="返回"
              className="col-start-1 row-start-1 inline-flex h-11 w-11 shrink-0 items-center justify-center rounded-full text-charcoal/70 transition-colors duration-200 hover:bg-sand/40 hover:text-charcoal active:scale-95"
            >
              <ArrowLeftIcon className="h-5 w-5" />
            </button>
            <h2 className="col-start-2 row-start-1 min-w-0 truncate font-serif text-xl leading-tight text-charcoal md:text-3xl">
              {initial ? '编辑单词' : '添加单词'}
            </h2>
            {(draftState || sealed) && (
              <p className="col-start-3 row-start-1 hidden justify-self-end whitespace-nowrap text-xs tabular-nums text-charcoal/40 md:block">
                {sealed ? '已保存' : '未保存 ' + changedFields + ' 处'}
              </p>
            )}
            <div className="col-start-3 row-start-1 flex items-center gap-2 justify-self-end md:col-start-4 md:gap-3">
              <button
                ref={dictTriggerRef}
                type="button"
                aria-expanded={dictOpen}
                aria-controls="word-dict-rail"
                onClick={() => setDictOpen((v) => !v)}
                className="inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded-full border border-charcoal/20 px-3.5 text-sm font-medium text-charcoal transition-all duration-200 hover:border-charcoal/40 hover:bg-white active:scale-95 md:px-4"
              >
                <GlobeIcon className="h-4 w-4" />
                <span>词典</span>
              </button>
              <button
                type="submit"
                form="word-form"
                disabled={saving}
                className={
                  'inline-flex min-h-11 shrink-0 items-center justify-center gap-1.5 rounded-full px-5 text-sm font-medium text-ivory shadow-lg shadow-charcoal/10 transition-all duration-200 active:scale-95 ' +
                  (sealed
                    ? 'bg-charcoal'
                    : 'bg-charcoal hover:bg-charcoal/90 disabled:bg-charcoal/20 disabled:text-charcoal/40 disabled:shadow-none')
                }
              >
                {sealed && <CheckIcon className="h-4 w-4 animate-fade-in" />}
                {primaryLabel}
              </button>
            </div>
          </div>
        </header>

        <main className="mx-auto flex min-h-0 w-full max-w-[1400px] flex-1 flex-col pt-1">
          {/* 回填状态：顶条下方一条独立状态带，逐字段的「撤销」都在这里 ——
              不再插在字段之间把下面的表单整体推下去；没有状态时不占高度（180ms 过渡）。 */}
          <div
            className={
              'flex shrink-0 items-center gap-2 overflow-hidden px-1 transition-[height,margin] duration-200 ' +
              (notes.word || notes.phonetic || notes.definitions || notes.example ? 'mb-2 h-9' : 'mb-0 h-0')
            }
          >
            {(['word', 'phonetic', 'definitions', 'example'] as const).map(
              (k) => notes[k] && <FillNoteRow key={k} note={notes[k]!} className="shrink-0" />,
            )}
          </div>
          {/* 两张卡同一个父级（.wb-body）：左纸卡 / 右词典白卡由 flex 拉伸成同高 ——
              top 与 bottom 完全对齐，桌面间距 20px；<768 折成「纸卡 42% / 把手 44 / 词典」的上下分屏。 */}
          <div className={'wb-body mb-2' + (dictOpen ? '' : ' is-solo')}>
            <div className="wb-sheet animate-fade-in-up">
              {/* 纸面：下层垫一张 #F2EAE0（位移 8px），四周留白让环境底图透出 */}
              <span
                aria-hidden="true"
                className="pointer-events-none absolute inset-0 translate-x-2 translate-y-2 rounded-2xl bg-[#F2EAE0]"
              />
              <div className="relative flex min-h-0 flex-1 flex-col rounded-2xl border border-charcoal/5 bg-[#FDFAF4] shadow-sm">
                <form id="word-form" className="wb-form" onSubmit={handleSubmit}>
                  <div className="wb-form-scroll px-4 pt-3 pb-4 md:px-5 md:pt-3 md:pb-4">
                    <div className="flex min-w-0 flex-col gap-4 md:gap-5">
                      {/* 词头：衬线大字直接坐在纸上（Bookshelf / WordTable 的字不套框），
                          底线是 clay，焦点时由 index.css 的 field-underline 加深。 */}
                      <section className="relative">
                        <label htmlFor="word-spelling" className="sr-only">
                          单词
                        </label>
                        <input
                          id="word-spelling"
                          ref={spellingRef}
                          value={spelling}
                          onChange={(e) => setSpelling(e.target.value)}
                          placeholder="abandon"
                          className="wb-headword field-underline block border-b border-clay/40 bg-transparent px-0 pb-1 font-serif text-[32px] leading-none tracking-tight text-charcoal caret-clay transition-colors duration-200 placeholder:text-charcoal/30 md:text-[40px]"
                        />
                        <label htmlFor="word-phonetic" className="sr-only">
                          音标
                        </label>
                        <input
                          id="word-phonetic"
                          ref={phoneticRef}
                          value={phonetic}
                          onChange={(e) => {
                            setPhonetic(e.target.value)
                            setNotes((n) => ({ ...n, phonetic: undefined }))
                          }}
                          placeholder="/əˈbændən/"
                          className={'mt-2.5 block w-full px-3 ' + FIELD}
                        />
                      </section>

                      {/* 释义账本：白卡 + divide-y，行内是词性 / 释义 / 44 删除 */}
                      <section>
                        <div className="flex items-baseline justify-between gap-3">
                          <h3 className={SECTION_TITLE}>释义</h3>
                          <span className={SECTION_COUNT}>{definitions.length} 条</span>
                        </div>
                        <div className="mt-2 overflow-hidden rounded-2xl border border-charcoal/5 bg-white shadow-sm">
                          <div
                            ref={definitionsRef}
                            tabIndex={-1}
                            className="divide-y divide-charcoal/5 px-2 outline-none"
                          >
                            {definitions.map((d, i) => (
                              <div
                                key={i}
                                style={{ animationDelay: Math.min(i, 6) * 30 + 'ms' }}
                                className="flex min-h-14 animate-word-rise items-center gap-2 rounded-xl px-1 py-1 transition-colors duration-200 hover:bg-sand/20"
                              >
                                {/* 边框画在外层 span 上、select 本身透明无边框：
                                    Safari / iOS 不会再往里画一层原生控件框 */}
                                <span className="relative flex shrink-0 items-center overflow-hidden rounded-xl border border-charcoal/15 bg-white transition-colors duration-200 focus-within:border-clay">
                                  <select
                                    value={d.pos}
                                    onChange={(e) => updateDef(i, { pos: e.target.value })}
                                    aria-label="词性"
                                    className={
                                      'min-h-11 w-[4.75rem] appearance-none border-0 bg-transparent pr-6 pl-2.5 text-sm focus:outline-none sm:w-[5.5rem] ' +
                                      (d.pos === '' ? 'text-charcoal/40' : 'text-charcoal')
                                    }
                                  >
                                    <option value="" disabled hidden>
                                      词性
                                    </option>
                                    {d.pos && !POS_OPTIONS.includes(d.pos) && <option value={d.pos}>{d.pos}</option>}
                                    {POS_OPTIONS.map((p) => (
                                      <option key={p} value={p}>
                                        {p}
                                      </option>
                                    ))}
                                  </select>
                                  <ChevronRightIcon className="pointer-events-none absolute right-1.5 h-3.5 w-3.5 rotate-90 text-charcoal/40" />
                                </span>
                                <input
                                  value={d.meaning}
                                  onChange={(e) => {
                                    updateDef(i, { meaning: e.target.value })
                                    setNotes((n) => ({ ...n, definitions: undefined }))
                                  }}
                                  aria-label={'释义 ' + (i + 1)}
                                  placeholder="释义内容"
                                  className={'min-w-0 flex-1 px-3 ' + FIELD}
                                />
                                {definitions.length > 1 && (
                                  <button
                                    type="button"
                                    onClick={() => removeDefinition(i)}
                                    aria-label="删除此释义"
                                    className={
                                      ICON_BTN +
                                      ' border border-charcoal/10 text-charcoal/40 hover:border-red-300 hover:text-red-400'
                                    }
                                  >
                                    <CloseIcon className="h-4 w-4" />
                                  </button>
                                )}
                              </div>
                            ))}
                          </div>
                          <div className="border-t border-charcoal/5">
                            <button
                              type="button"
                              onClick={addDefinition}
                              className="flex min-h-12 w-full items-center gap-2 px-3.5 text-sm font-medium text-charcoal/60 transition-colors duration-200 hover:bg-sand/20 hover:text-charcoal"
                            >
                              <PlusIcon className="h-4 w-4" />
                              添加释义
                            </button>
                          </div>
                        </div>
                      </section>

                      {/* 例句 */}
                      <section>
                        <h3 className={SECTION_TITLE}>例句</h3>
                        <label htmlFor="word-example" className="sr-only">
                          例句
                        </label>
                        <textarea
                          id="word-example"
                          ref={exampleRef}
                          value={example}
                          onChange={(e) => {
                            setExample(e.target.value)
                            setNotes((n) => ({ ...n, example: undefined }))
                          }}
                          rows={3}
                          className="mt-2 block min-h-24 w-full resize-none rounded-xl border border-charcoal/15 bg-white px-3 py-2.5 text-sm leading-relaxed text-charcoal transition-colors duration-200 focus:border-clay"
                        />
                      </section>

                      {/* 标签（TagPicker 自带项目胶囊：sand 底 / 选中 charcoal，命中区 44） */}
                      <section>
                        <div className="flex items-baseline justify-between gap-3">
                          <h3 className={SECTION_TITLE}>标签</h3>
                          <span className={SECTION_COUNT}>{tagDraft.selected.size} 个</span>
                        </div>
                        <div className="mt-1.5 min-w-0">
                          <TagPicker
                            tags={tags}
                            pending={tagDraft.pending}
                            selected={tagDraft.selected}
                            onToggle={tagDraft.toggle}
                            onCreate={tagDraft.create}
                            disabled={saving}
                          />
                        </div>
                      </section>
                    </div>
                  </div>

                  {/* 错误：纸面内恒定一行（换文案不推挤版面） */}
                  <div className="min-h-6 shrink-0 px-4 pb-1 md:px-5" role="status" aria-live="polite">
                    {error && <p className="text-sm text-red-600">{error}</p>}
                  </div>
                </form>
              </div>
            </div>

            {/* 窄屏：词典从底部升起（遮罩 + 白卡），不再和表单上下分屏 */}
            {rail.mounted && (
              <div
                className="wb-scrim md:hidden"
                aria-hidden="true"
                data-state={dictOpen ? 'open' : 'closing'}
                onClick={closeDict}
              />
            )}

            {/* 词典栏：桌面与纸卡同高的白纸卡；<768 是覆盖面板（内容由 DictSearchPanel rail 填）。
                面板**常驻挂载**（收起后已查的词/勾选/例句展开状态都要保留，回去接着加不用重搜；
                导航按钮的 aria-controls 也一直指向真实节点），只靠 hidden + data-state 控制显隐与动画：
                收起先播退场（桌面收列宽 + 右移淡出，表单卡同步变宽；窄屏整张滑下去），260ms 后才 hidden。 */}
            <aside
              id="word-dict-rail"
              className="wb-rail"
              hidden={!rail.mounted}
              data-state={dictOpen ? (rail.entering ? 'entering' : 'open') : 'closing'}
            >
              <DictSearchPanel
                initialWord={spelling}
                visible={dictOpen}
                rail
                onApply={handleDictApply}
                onBack={closeDict}
              />
            </aside>
          </div>
        </main>
      </div>

      {guard.confirmOpen && (
        <DiscardConfirmModal onDiscard={guard.confirmDiscard} onCancel={guard.cancelDiscard} />
      )}
    </Modal>
  )
}
