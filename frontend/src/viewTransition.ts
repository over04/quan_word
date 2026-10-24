import { flushSync } from 'react-dom'

let active: ViewTransition | null = null
/** 过渡期间排队的变更（连续调用不丢，settle 时按序合并执行） */
let pending: Array<() => void> = []
/** 被合并进后续过渡的调用方，等在它们那次过渡结束后一起 resolve */
let pendingResolvers: Array<() => void> = []
/** 队列里累积的"页面级切换"需求（任一为 true 即整条合并过渡都算页面级） */
let pendingNavMotion = false

/** 过渡进行中的根标记：motion 的 CSS 只在 `html.vt-running` 期间把入场动画快进到终态（不销毁动画） */
const RUNNING_CLASS = 'vt-running'

/**
 * 页面级切换标记（首页 ↔ 书页、全屏词典详情进出）：标题栏要"退出 / 落位"。
 * 只有整页换了才加 —— 分区切换、纸质/列表切换这类同页变化里导航原地不动，
 * 给它做淡出淡入只会让"切个 tab 整条栏闪一下"。
 */
const NAV_MOTION_CLASS = 'vt-nav-motion'

/** 本次转场要共享飞行的词（只有它挂 view-transition-name，避免几十个命名元素拖垮转场） */
let transitionWord: string | null = null

/**
 * 与 index.css 里 `html.vt-running :is(...)` 的快进名单**逐项一致**（改一边必须同步另一边）。
 * 转场结束时要把这些入场动画「用掉」：摘掉类名，元素停在自然终态。
 */
const ENTRANCE_CLASSES = [
  'animate-hero-in',
  'animate-detail-in',
  'animate-page-in',
  'animate-result-in',
  'animate-step-in',
  'animate-fade-in',
  'animate-fade-in-up',
  'animate-bar-down',
  'animate-nav-in',
  'animate-book-rise',
  'animate-word-rise',
] as const

/**
 * 转场结束：把这次转场里被快进过的入场动画**摘掉**（不是留着）。
 *
 * 为什么必须摘：`html.vt-running` 的快进只是把 duration 压到 0.001s 并清掉延迟。
 * 转场结束时摘掉 vt-running，duration 恢复真实值，浏览器会按「元素上仍挂着这条动画」
 * 重新建一个 CSSAnimation，currentTime 按真实经过时间算 —— 于是入场动画在转场结束后
 * 又播一遍（书架卡片/列表项"加载完成抽搐"就是这么来的；`finish()` 也救不了，因为
 * 重建出来的是**新对象**）。类名摘掉后元素不再声明这条动画，自然没有第二遍。
 *
 * 只在 settle 这一刻处理：之后才挂载的内容（异步数据、弹窗、列表项）照常播放入场动画。
 */
function consumeEntranceAnimations() {
  const selector = ENTRANCE_CLASSES.map((name) => '.' + name).join(',')
  for (const element of document.querySelectorAll(selector)) {
    for (const name of ENTRANCE_CLASSES) element.classList.remove(name)
  }
}

/** 登记本次转场要共享飞行的词；必须在开转场前完成渲染（调用方 flushSync 一次）。 */
export function setTransitionWord(word: string | null) {
  transitionWord = word === null ? null : word.trim().toLowerCase()
}

/**
 * 串行化 View Transition：**skip + 队列**（唯一一套机制）。
 * - 已有过渡在跑：把本次变更 push 进 pending 队列，并 skipTransition() 打断当前过渡；
 *   被跳过的过渡 finished 会 reject → settle 立刻用「队列里累积的全部变更」开一条新过渡。
 *   这样既不会把变更拖到旧过渡的 0.5s 之后（排队等待会表现为"内容半天不换"），也不会丢变更
 *   （单槽实现会把中间的变更覆盖掉）。
 * - 过渡期间给 <html> 挂 `vt-running`（入场动画由 CSS 快进到终态），settle 时摘掉；
 *   队列非空时 vt-running 继续挂着，避免类名抖动。
 * 返回的 Promise 在包含本次变更的过渡结束后 resolve；不支持 VT 时同步执行并立即 resolve。
 */
export function withViewTransition(change: () => void, opts?: { navMotion?: boolean }): Promise<void> {
  const navMotion = opts?.navMotion === true
  if (!document.startViewTransition) {
    // 没有 VT 的浏览器：直接改 DOM，入场动画照常播
    change()
    return Promise.resolve()
  }
  if (active) {
    // 入队 + 跳过当前过渡：skipTransition() 让 finished reject → settle 立刻用队列开新过渡，
    // 既不等待旧过渡跑完（否则内容半天不换），也不丢中间的变更。
    pending.push(change)
    if (navMotion) pendingNavMotion = true
    active.skipTransition()
    return new Promise<void>((resolve) => {
      pendingResolvers.push(resolve)
    })
  }
  return start(change, navMotion)
}

function start(change: () => void, navMotion: boolean): Promise<void> {
  document.documentElement.classList.add(RUNNING_CLASS)
  document.documentElement.classList.toggle(NAV_MOTION_CLASS, navMotion)
  const transition = document.startViewTransition(() => flushSync(change))
  active = transition
  return new Promise<void>((resolve) => {
    const settle = () => {
      if (active === transition) active = null
      const queued = pending
      pending = []
      if (queued.length > 0) {
        // 合并后的变更单独跑一条过渡；vt-running 持续挂着，避免类名抖动
        const merged = pendingResolvers
        pendingResolvers = []
        const mergedNavMotion = pendingNavMotion
        pendingNavMotion = false
        void start(() => {
          for (const fn of queued) fn()
        }, mergedNavMotion).then(() => {
          for (const done of merged) done()
        })
      } else {
        // 先"用掉"快进过的入场动画，再摘 vt-running：否则类名一摘它们会重播一遍
        consumeEntranceAnimations()
        document.documentElement.classList.remove(RUNNING_CLASS)
        document.documentElement.classList.remove(NAV_MOTION_CLASS)
        pendingNavMotion = false
      }
      resolve()
    }
    // finished 在过渡被跳过/中断时会 reject（AbortError）：两个分支都必须走 settle，
    // 否则会出现未处理的 Promise 拒绝，并且 vt-running 永远摘不掉。
    transition.finished.then(settle, settle)
  })
}

/** 生成稳定的 view-transition-name：让结果词与详情标题共享同一条移动动画。 */
export function wordTransitionName(word: string): string {
  const slug = word
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  return `dict-word-${slug || 'word'}`
}

/** 只有「这次要飞的那个词」才返回名字，其余返回 undefined（不参与转场）。 */
export function transitionNameFor(word: string): string | undefined {
  if (transitionWord === null || transitionWord !== word.trim().toLowerCase()) return undefined
  return wordTransitionName(word)
}