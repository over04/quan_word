import { ChevronLeftIcon, ChevronRightIcon } from './Icons'

interface Props {
  page: number
  totalPages: number
  busy?: boolean
  onPrev: () => void
  onNext: () => void
}

export default function Pagination({ page, totalPages, busy, onPrev, onNext }: Props) {
  if (totalPages <= 1) return null
  return (
    <div className="flex items-center justify-center gap-5 py-10">
      <button
        onClick={onPrev}
        disabled={page <= 1 || busy}
        className="inline-flex items-center gap-1.5 px-5 py-2.5 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all disabled:opacity-35 disabled:hover:bg-transparent disabled:cursor-not-allowed"
      >
        <ChevronLeftIcon className="w-4 h-4" />
        上一页
      </button>
      <span className="text-sm text-charcoal/50 tabular-nums">
        第 <span className="font-semibold text-charcoal">{page}</span> / {totalPages} 页
      </span>
      <button
        onClick={onNext}
        disabled={page >= totalPages || busy}
        className="inline-flex items-center gap-1.5 px-5 py-2.5 rounded-full bg-charcoal text-ivory text-sm font-medium hover:bg-charcoal/90 transition-all shadow-lg shadow-charcoal/10 disabled:opacity-35 disabled:hover:bg-charcoal disabled:cursor-not-allowed"
      >
        下一页
        <ChevronRightIcon className="w-4 h-4" />
      </button>
    </div>
  )
}
