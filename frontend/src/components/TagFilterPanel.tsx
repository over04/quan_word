import { sanitizeTagFilter, type Tag, type TagFilter, type TagFilterGroup } from '../api'

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

/** 组内 toggle 一个标签（深拷贝局部更新；无标签组不可选标签；空筛选时创建组并选中） */
function toggleGroupTag(filter: TagFilter, gi: number, id: number): TagFilter {
  if (filter.groups.length === 0) {
    return { groups: [{ mode: 'and', ids: [id] }], links: [] }
  }
  const g = filter.groups[gi]
  if (g.mode === 'none') return filter
  const ids = g.ids.includes(id) ? g.ids.filter((x) => x !== id) : [...g.ids, id]
  return {
    groups: filter.groups.map((grp, i) => (i === gi ? { ...grp, ids } : grp)),
    links: filter.links,
  }
}

/** 切换组匹配方式；切到无标签时清空该组标签选择（后端拒绝 none 组带 ids） */
function setGroupMode(filter: TagFilter, gi: number, mode: Mode): TagFilter {
  return {
    groups: filter.groups.map((grp, i) =>
      i === gi ? (mode === 'none' ? { mode, ids: [] } : { ...grp, mode }) : grp,
    ),
    links: filter.links,
  }
}

/** 设置组 gi 与组 gi+1 之间的连接词 */
function setLink(filter: TagFilter, gi: number, link: Link): TagFilter {
  return {
    groups: filter.groups,
    links: filter.links.map((l, i) => (i === gi ? link : l)),
  }
}

function addGroup(filter: TagFilter): TagFilter {
  return {
    groups: [...filter.groups, { mode: 'and', ids: [] }],
    links: [...filter.links, 'and'],
  }
}

/** 删除组 gi：连接词同步收缩（非最后组取右侧连接，最后组删掉左侧连接） */
function removeGroup(filter: TagFilter, gi: number): TagFilter {
  return {
    groups: filter.groups.filter((_, i) => i !== gi),
    links: filter.links.filter((_, i) =>
      gi === filter.groups.length - 1 ? i !== gi - 1 : i !== gi,
    ),
  }
}

/** 单张组卡片：组头（序号 + 删除按钮）+ 匹配方式三态 + 标签 chips */
function GroupCard({
  group,
  gi,
  siblings,
  tags,
  onChange,
}: {
  group: TagFilterGroup
  gi: number
  siblings: number
  tags: Tag[]
  onChange: (updater: (prev: TagFilter) => TagFilter) => void
}) {
  return (
    <div className="rounded-xl border border-charcoal/10 p-2.5">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-charcoal/40 shrink-0">组 {gi + 1}</span>
        {siblings > 1 && (
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
            aria-pressed={group.mode === m}
            className={`inline-flex items-center justify-center px-1.5 py-1 rounded-md text-xs font-medium transition-colors ${
              group.mode === m
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
            aria-pressed={group.ids.includes(t.id)}
            aria-disabled={group.mode === 'none'}
            tabIndex={group.mode === 'none' ? -1 : 0}
            className={`px-2.5 py-1 rounded-full text-xs border transition-colors ${
              group.mode === 'none'
                ? 'border-charcoal/10 text-charcoal/30 cursor-not-allowed'
                : group.ids.includes(t.id)
                  ? 'bg-charcoal text-ivory border-charcoal'
                  : 'border-charcoal/15 text-charcoal/70 hover:bg-sand/60'
            }`}
          >
            {t.name}
          </button>
        ))}
      </div>
    </div>
  )
}

/** 标签筛选面板：平铺条件组编辑（组内 and/or/none，组间且/或连接）。
 *  优先级（「且」优先于「或」）用分段视觉直观体现：按「或」把组切成段，
 *  段容器用边框/背景包裹（括号感），段间「或」为醒目大按钮，段内「且」为小按钮。 */
export default function TagFilterPanel({ tags, filter, onChange, onClose }: Props) {
  // 净化后仍有有效条件（含无标签组）即视为筛选生效；「全部单词」仅在真正无筛选时高亮
  const isEmpty = sanitizeTagFilter(filter).groups.length === 0
  // 按「或」链接切段：每段 = 组索引数组（段内连接词全为且）
  const segments: number[][] = []
  let cur: number[] = []
  filter.groups.forEach((_, gi) => {
    cur.push(gi)
    if (gi + 1 < filter.groups.length && filter.links[gi] === 'or') {
      segments.push(cur)
      cur = []
    }
  })
  if (cur.length > 0) segments.push(cur)

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
      <div className="mt-3 space-y-1.5">
        {filter.groups.length === 0 ? (
          /* 空态：未筛选提示 + 全量标签 chips，点标签即建组并选中 */
          <div className="rounded-xl border border-dashed border-charcoal/15 p-3">
            <p className="text-center text-xs text-charcoal/40">未筛选 · 点击标签开始筛选</p>
            <div className="mt-2 flex flex-wrap gap-1.5 justify-center">
              {tags.map((t) => (
                <button
                  key={t.id}
                  onClick={() => onChange((prev) => toggleGroupTag(prev, 0, t.id))}
                  className="px-2.5 py-1 rounded-full text-xs border border-charcoal/15 text-charcoal/70 hover:bg-sand/60 transition-colors"
                >
                  {t.name}
                </button>
              ))}
            </div>
          </div>
        ) : (
          segments.map((seg, si) => (
            <div key={si}>
              {/* 段间「或」：醒目大按钮 + 左右分隔线（优先级边界） */}
              {si > 0 && (
                <div className="flex items-center gap-3 py-1" role="tablist" aria-label={`组 ${seg[0]} 与组 ${seg[0] + 1} 之间的连接方式`}>
                  <div className="h-px flex-1 bg-charcoal/15" />
                  <button
                    onClick={() => onChange((prev) => setLink(prev, seg[0] - 1, 'and'))}
                    aria-pressed
                    className="w-9 h-9 rounded-full bg-charcoal text-ivory text-sm font-bold shadow-md hover:bg-charcoal/85 transition-colors shrink-0"
                  >
                    或
                  </button>
                  <div className="h-px flex-1 bg-charcoal/15" />
                </div>
              )}
              {/* 段容器：边框/背景包裹，括号感；段内连接词全为「且」 */}
              <div className="rounded-xl border border-charcoal/10 bg-sand/10 p-2.5 space-y-1.5">
                {seg.map((gi, i) => (
                  <div key={gi}>
                    {i > 0 && (
                      <div className="flex items-center justify-center py-0.5" role="tablist" aria-label={`组 ${gi} 与组 ${gi + 1} 之间的连接方式`}>
                        <button
                          onClick={() => onChange((prev) => setLink(prev, gi - 1, 'or'))}
                          aria-pressed
                          className="px-3 py-0.5 rounded-full bg-sand/60 text-charcoal/80 text-xs font-semibold hover:bg-sand transition-colors"
                        >
                          且
                        </button>
                      </div>
                    )}
                    <GroupCard group={filter.groups[gi]} gi={gi} siblings={filter.groups.length} tags={tags} onChange={onChange} />
                  </div>
                ))}
              </div>
            </div>
          ))
        )}
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
