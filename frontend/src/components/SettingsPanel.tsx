import { useState } from 'react'

interface Props {
  pageSize: number
  fontScale: number
  onChangePageSize: (n: number) => void
  onChangeFontScale: (s: number) => void
  onClose: () => void
}

const PAGE_SIZE_MIN = 10
const PAGE_SIZE_MAX = 200
const FONT_MIN = 12
const FONT_MAX = 28

/** 阅读设置面板：每页单词数（滑块）+ 字号（滑块） */
export default function SettingsPanel({
  pageSize,
  fontScale,
  onChangePageSize,
  onChangeFontScale,
  onClose,
}: Props) {
  // 滑块拖动中的临时值：拖动结束（松开/键盘）才应用
  const [draftSize, setDraftSize] = useState(pageSize)
  const [draftFont, setDraftFont] = useState(fontScale)

  return (
    <div className="absolute right-0 top-12 z-50 w-72 bg-white rounded-2xl border border-charcoal/10 shadow-xl shadow-charcoal/10 p-5 animate-fade-in-up">
      <div className="flex items-center justify-between">
        <p className="font-serif text-base text-charcoal">阅读设置</p>
        <button
          onClick={onClose}
          className="w-7 h-7 rounded-full text-charcoal/40 hover:bg-sand/40 hover:text-charcoal transition-colors"
          aria-label="关闭设置"
        >
          ✕
        </button>
      </div>

      <div className="mt-4">
        <div className="flex items-center justify-between">
          <p className="text-xs font-bold text-charcoal/50 uppercase tracking-widest">每页单词数</p>
          <span className="text-sm font-semibold text-charcoal tabular-nums">{draftSize}</span>
        </div>
        <input
          type="range"
          min={PAGE_SIZE_MIN}
          max={PAGE_SIZE_MAX}
          step={10}
          value={draftSize}
          onChange={(e) => setDraftSize(Number(e.target.value))}
          onPointerUp={() => onChangePageSize(draftSize)}
          onKeyUp={() => onChangePageSize(draftSize)}
          className="mt-3 w-full accent-clay cursor-pointer"
          aria-label="每页单词数"
        />
        <div className="mt-1 flex justify-between text-[11px] text-charcoal/35 tabular-nums">
          <span>{PAGE_SIZE_MIN}</span>
          <span>{PAGE_SIZE_MAX}</span>
        </div>
      </div>

      <div className="mt-4">
        <div className="flex items-center justify-between">
          <p className="text-xs font-bold text-charcoal/50 uppercase tracking-widest">字号</p>
          <span className="text-sm font-semibold text-charcoal tabular-nums">{draftFont}px</span>
        </div>
        <input
          type="range"
          min={FONT_MIN}
          max={FONT_MAX}
          step={1}
          value={draftFont}
          onChange={(e) => setDraftFont(Number(e.target.value))}
          onPointerUp={() => onChangeFontScale(draftFont)}
          onKeyUp={() => onChangeFontScale(draftFont)}
          className="mt-3 w-full accent-clay cursor-pointer"
          aria-label="字号"
        />
        <div className="mt-1 flex justify-between text-[11px] text-charcoal/35 tabular-nums">
          <span>{FONT_MIN}px</span>
          <span>{FONT_MAX}px</span>
        </div>
      </div>
    </div>
  )
}
