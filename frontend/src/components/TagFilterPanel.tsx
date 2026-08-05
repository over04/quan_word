import type { Tag } from '../api'
import type { TagFilter } from '../api'

interface Props {
  /** 该书全部标签（每组均展示全量 chips，选中状态按组独立） */
  tags: Tag[]
  /** 当前筛选：组数组 + 组间连接词（links[i] = 组 i 与组 i+1 之间） */
  filter: TagFilter
  /** 任一操作后的更新函数（函数式更新，连续操作按序应用不互相覆盖） */
  onChange: (updater: (prev: TagFilter) => TagFilter) => void
  onClose: () => void
}

type Link = 'and' | 'or'
type Mode = 'and' | 'or' | 'none'

/** 空筛选 = 一个未选中的默认组（and），操作以它为起点 */
function normalize(filter: TagFilter): TagFilter {
  return filter.groups.length > 0 ? filter : { groups: [{ mode: 'and', ids: [] }], links: [] }
}

/** 组内 toggle 一个标签（深拷贝局部更新；无标签组不可选标签） */
function toggleGroupTag(filter: TagFilter, gi: number, id: number): TagFilter {
  const base = normalize(filter)
  const g = base.groups[gi]
  if (g.mode === 'none') return filter
  const ids = g.ids.includes(id) ? g.ids.filter((x) => x !== id) : [...g.ids, id]
  return {
    groups: base.groups.map((grp, i) => (i === gi ? { ...grp, ids } : grp)),
    links: base.links,
  }
}

function setGroupMode(filter: TagFilter, gi: number, mode: Mode): TagFilter {
  const base = normalize(filter)
  return {
    // 切到无标签时清空该组标签选择（后端拒绝 none 组带 ids）
    groups: base.groups.map((grp, i) =>
      i === gi ? (mode === 'none' ? { mode, ids: [] } : { ...grp, mode }) : grp,
    ),
    links: base.links,
  }
}

/** 设置组 gi 与组 gi+1 之间的连接词 */
function setLink(filter: TagFilter, gi: number, link: Link): TagFilter {
  const base = normalize(filter)
  return {
    groups: base.groups,
    links: base.links.map((l, i) => (i === gi ? link : l)),
  }
}

function addGroup(filter: TagFilter): TagFilter {
  const base = normalize(filter)
  return {
    groups: [...base.groups, { mode: 'and', ids: [] }],
    links: [...base.links, 'and'],
  }
}

/** 删除组 gi：连接词同步收缩（非最后组取右侧连接，最后组删掉左侧连接） */
function removeGroup(filter: TagFilter, gi: number): TagFilter {
  const base = normalize(filter)
  return {
    groups: base.groups.filter((_, i) => i !== gi),
    links: base.links.filter((_, i) =>
      gi === base.groups.length - 1 ? i !== gi - 1 : i !== gi,
    ),
  }
}

/** 标签筛选面板：分组条件编辑（组内 and/or/none，组间且/或连接）。受控组件，全部操作走 onChange。 */
export default function TagFilterPanel({ tags, filter, onChange, onClose }: Props) {
  // 渲染用展示态：空筛选显示一个默认空组（组 1）
  const display: TagFilter =
    filter.groups.length > 0 ? filter : { groups: [{ mode: 'and', ids: [] }], links: [] }
  const isEmpty = display.groups.every((g) => g.ids.length === 0)
  return (
    <div className="absolute right-0 top-12 z-50 w-72 max-w-[calc(100vw-2rem)] max-h-96 overflow-y-auto bg-white rounded-2xl border border-charcoal/10 shadow-xl shadow-charcoal/10 p-4 animate-fade-in-up">
      <div className="flex items-center justify-between">
        <p className="font-serif text-base text-charcoal">按标签筛选</p>
        <button
          onClick={onClose}
          className="w-7 h-7 rounded-full text-charcoal/40 hover:bg-sand/40 hover:text-charcoal transition-colors"
          aria-label="关闭筛选"
        >
          ✕
        </button>
      </div>
      <div className="mt-3 space-y-2">
        {display.groups.map((g, gi) => (
          <div key={gi}>
            {gi > 0 && (
              <div className="flex items-center justify-center py-1">
                <div
                  className="grid grid-cols-2 gap-0.5 rounded-lg bg-sand/30 p-0.5"
                  role="tablist"
                  aria-label={`组 ${gi} 与组 ${gi + 1} 之间的连接方式`}
                >
                  {(['and', 'or'] as const).map((l) => (
                    <button
                      key={l}
                      onClick={() => onChange((prev) => setLink(prev, gi - 1, l))}
                      aria-pressed={(display.links[gi - 1] ?? 'and') === l}
                      className={`px-2.5 py-0.5 rounded-md text-xs font-medium transition-colors ${
                        (display.links[gi - 1] ?? 'and') === l
                          ? 'bg-charcoal text-ivory shadow-md'
                          : 'text-charcoal/70 hover:bg-sand/60'
                      }`}
                    >
                      {l === 'and' ? '且' : '或'}
                    </button>
                  ))}
                </div>
              </div>
            )}
            <div className="rounded-xl border border-charcoal/10 p-2.5">
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs text-charcoal/40 shrink-0">组 {gi + 1}</span>
                {display.groups.length > 1 && (
                  <button
                    onClick={() => onChange((prev) => removeGroup(prev, gi))}
                    className="w-6 h-6 shrink-0 rounded-full text-charcoal/40 hover:bg-sand/40 hover:text-charcoal transition-colors"
                    aria-label={`删除组 ${gi + 1}`}
                  >
                    ✕
                  </button>
                )}
              </div>
              {/* 组内匹配方式：全部匹配（交集）/ 任一匹配（并集）/ 无标签；独占一行避免窄面板换行 */}
              <div
                className="mt-1.5 grid grid-cols-3 gap-0.5 rounded-lg bg-sand/30 p-0.5"
                role="tablist"
                aria-label={`组 ${gi + 1} 匹配方式`}
              >
                {(
                  [
                    ['and', '全部匹配'],
                    ['or', '任一匹配'],
                    ['none', '无标签'],
                  ] as const
                ).map(([m, label]) => (
                  <button
                    key={m}
                    onClick={() => onChange((prev) => setGroupMode(prev, gi, m))}
                    aria-pressed={g.mode === m}
                    className={`inline-flex items-center justify-center px-1.5 py-1 rounded-md text-xs font-medium transition-colors ${
                      g.mode === m
                        ? 'bg-charcoal text-ivory shadow-md'
                        : 'text-charcoal/70 hover:bg-sand/60'
                    }`}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <div className="mt-2 flex flex-wrap gap-1.5">
                {tags.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => onChange((prev) => toggleGroupTag(prev, gi, t.id))}
                    aria-pressed={g.ids.includes(t.id)}
                    aria-disabled={g.mode === 'none'}
                    tabIndex={g.mode === 'none' ? -1 : 0}
                    className={`px-2.5 py-1 rounded-full text-xs border transition-colors ${
                      g.mode === 'none'
                        ? 'border-charcoal/10 text-charcoal/30 cursor-not-allowed'
                        : g.ids.includes(t.id)
                          ? 'bg-charcoal text-ivory border-charcoal'
                          : 'border-charcoal/15 text-charcoal/70 hover:bg-sand/60'
                    }`}
                  >
                    {t.name}
                  </button>
                ))}
              </div>
            </div>
          </div>
        ))}
      </div>
      <button
        onClick={() => onChange(addGroup)}
        className="mt-3 w-full py-2 rounded-xl border border-dashed border-charcoal/25 text-sm text-charcoal/60 hover:border-charcoal/50 hover:text-charcoal transition-colors"
      >
        + 添加条件组
      </button>
      <div className="mt-3">
        <button
          onClick={() => onChange(() => ({ groups: [], links: [] }))}
          aria-pressed={isEmpty}
          className={`w-full flex items-center justify-between px-3 py-2 rounded-xl text-sm transition-colors ${
            isEmpty ? 'bg-charcoal text-ivory' : 'text-charcoal/70 hover:bg-sand/40'
          }`}
        >
          <span>全部单词</span>
        </button>
      </div>
    </div>
  )
}
