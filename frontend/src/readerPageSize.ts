export const MIN_READER_PAGE_SIZE = 10
// 与后端 parse_paging 的上限一致。
export const MAX_READER_PAGE_SIZE = 100
export const DEFAULT_READER_PAGE_SIZE = 30

export function loadReaderPageSize(): number {
  const stored = Number(localStorage.getItem('qw_page_size'))
  if (!Number.isInteger(stored) || stored < MIN_READER_PAGE_SIZE) return DEFAULT_READER_PAGE_SIZE
  return Math.min(stored, MAX_READER_PAGE_SIZE)
}
